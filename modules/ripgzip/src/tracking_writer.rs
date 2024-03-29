use std::cmp::min;
use std::collections::VecDeque;
use std::io::{self, Write};

use anyhow::{bail, Result};

use crc::crc32::Digest;
use crc::{crc32, Hasher32};

////////////////////////////////////////////////////////////////////////////////

const HISTORY_SIZE: usize = 32768;

pub struct TrackingWriter<T> {
    inner: T,
    len: usize,
    history: VecDeque<u8>,
    digest: Digest,
}

impl<T: Write> Write for TrackingWriter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let res = self.inner.write(buf)?;
        let slice_to_write = &buf[0..res];
        for e in slice_to_write {
            if self.history.len() == HISTORY_SIZE {
                self.history.pop_front();
            }
            self.history.push_back(*e);
        }
        <Digest as Hasher32>::write(&mut self.digest, slice_to_write);
        self.len += res;
        Ok(res)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.len = 0;
        self.history = VecDeque::with_capacity(HISTORY_SIZE);
        self.digest = Digest::new(crc32::IEEE);
        self.inner.flush()
    }
}

impl<T: Write> TrackingWriter<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            len: 0,
            history: VecDeque::with_capacity(HISTORY_SIZE),
            digest: Digest::new(crc32::IEEE),
        }
    }

    /// Write a sequence of `len` bytes written `dist` bytes ago.
    pub fn write_previous(&mut self, dist: usize, len: usize) -> Result<()> {
        if dist > self.history.len() {
            bail!("History size is less than offset")
        }
        let start = self.history.len() - dist;
        self.history.make_contiguous();
        let (slice, _) = self.history.as_slices();
        let mut repeating_vec = Vec::with_capacity(len);
        let right_border = min(slice.len(), start + len) - 1;
        let mut index = start;
        while repeating_vec.len() < len {
            repeating_vec.push(slice[index]);
            if index == right_border {
                index = start;
            } else {
                index += 1;
            }
        }
        let write = self.write(&repeating_vec)?;
        if write < len {
            bail!("Not all bytes were written")
        }
        Ok(())
    }

    pub fn byte_count(&self) -> usize {
        self.len
    }

    pub fn crc32(&self) -> u32 {
        self.digest.sum32()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write() -> Result<()> {
        let mut buf: &mut [u8] = &mut [0u8; 10];
        let mut writer = TrackingWriter::new(&mut buf);

        assert_eq!(writer.write(&[1, 2, 3, 4])?, 4);
        assert_eq!(writer.byte_count(), 4);
        assert_eq!(writer.crc32(), 3057449933);

        assert_eq!(writer.write(&[4, 8, 15, 16, 23])?, 5);
        assert_eq!(writer.byte_count(), 9);
        assert_eq!(writer.crc32(), 3948347807);

        assert_eq!(writer.write(&[0, 0, 123])?, 1);
        assert_eq!(writer.byte_count(), 10);
        assert_eq!(writer.crc32(), 2992191065);

        assert_eq!(writer.write(&[42, 124, 234, 27])?, 0);
        assert_eq!(writer.byte_count(), 10);
        assert_eq!(writer.crc32(), 2992191065);

        Ok(())
    }

    #[test]
    fn write_previous() -> Result<()> {
        let mut buf: &mut [u8] = &mut [0u8; 512];
        let mut writer = TrackingWriter::new(&mut buf);

        for i in 0..=255 {
            writer.write_u8(i)?;
        }

        writer.write_previous(192, 128)?;
        assert_eq!(writer.byte_count(), 384);
        assert_eq!(writer.crc32(), 2611529849);

        assert!(writer.write_previous(10000, 20).is_err());
        assert_eq!(writer.byte_count(), 384);
        assert_eq!(writer.crc32(), 2611529849);

        assert!(writer.write_previous(256, 256).is_err());
        assert_eq!(writer.byte_count(), 512);
        assert_eq!(writer.crc32(), 2733545866);

        assert!(writer.write_previous(1, 1).is_err());
        assert_eq!(writer.byte_count(), 512);
        assert_eq!(writer.crc32(), 2733545866);

        Ok(())
    }
}
