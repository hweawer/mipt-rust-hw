use byteorder::ReadBytesExt;
#[cfg(not(test))]
use log::debug;
use std::cmp::min;
use std::{
    fmt,
    io::{self, BufRead},
};

#[cfg(test)]
use std::println as debug;

////////////////////////////////////////////////////////////////////////////////
const MASK: u8 = 0b11111111;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BitSequence {
    bits: u16,
    len: u8,
}

impl BitSequence {
    pub fn new(bits: u16, len: u8) -> Self {
        Self { bits, len }
    }

    pub fn bits(&self) -> u16 {
        self.bits
    }

    pub fn len(&self) -> u8 {
        self.len
    }

    pub fn concat(self, other: Self) -> Self {
        assert!(
            self.len + other.len() <= 16,
            "Concatenation result of two bit sequences is larger than 16"
        );
        let bits;
        let len;
        if self.len == 0 {
            bits = other.bits;
            len = other.len;
        } else if other.len == 0 {
            bits = self.bits;
            len = self.len;
        } else {
            bits = self.bits << other.len | other.bits();
            len = self.len + other.len;
        }
        Self { bits, len }
    }
}

////////////////////////////////////////////////////////////////////////////////

struct BitReaderBuffer {
    buffer: u8,
    len: u8,
}

pub struct BitReader<T> {
    stream: T,
    bit_reader_buffer: BitReaderBuffer,
}

impl<T: BufRead> BitReader<T> {
    pub fn new(stream: T) -> Self {
        Self {
            stream,
            bit_reader_buffer: BitReaderBuffer { buffer: 0, len: 0 },
        }
    }

    pub fn read_bits(&mut self, len: u8) -> io::Result<BitSequence> {
        assert!(len <= 16, "You can only read up to 16 bits at a time");
        if len == 0 {
            return Ok(BitSequence::new(0, 0));
        }
        let mut seq: Option<BitSequence> = None;
        let mut size = len;
        while size > 0 {
            let mut shift = if size >= 8 { 8 } else { size % 8 };
            let save: u8;
            if self.bit_reader_buffer.len == 0 {
                let read = self.stream.read_u8()?;
                save = read & MASK >> (8 - shift);

                let new_buffer = read.checked_shr(shift as u32).unwrap_or(0);
                self.bit_reader_buffer.buffer = new_buffer;
                self.bit_reader_buffer.len = 8 - shift;
            } else {
                shift = min(self.bit_reader_buffer.len.clone(), shift);
                save = self.bit_reader_buffer.buffer & MASK >> (8 - shift);

                let new_buffer = self.bit_reader_buffer.buffer >> shift;
                self.bit_reader_buffer.buffer = new_buffer;
                self.bit_reader_buffer.len -= shift;
            }
            let new_seq = BitSequence::new(save as u16, shift);
            seq = seq.map(|s| new_seq.concat(s)).or(Some(new_seq));
            size -= shift;
        }
        Ok(seq.unwrap())
    }

    /// Discard all the unread bits in the current byte and return a mutable reference
    /// to the underlying reader.
    pub fn borrow_reader_from_boundary(&mut self) -> &mut T {
        self.bit_reader_buffer.len = 0;
        &mut self.stream
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_bits() -> io::Result<()> {
        let data: &[u8] = &[0b01100011, 0b11011011, 0b10101111];
        let mut reader = BitReader::new(data);
        assert_eq!(reader.read_bits(1)?, BitSequence::new(0b1, 1));
        assert_eq!(reader.read_bits(2)?, BitSequence::new(0b01, 2));
        assert_eq!(reader.read_bits(3)?, BitSequence::new(0b100, 3));
        assert_eq!(reader.read_bits(4)?, BitSequence::new(0b1101, 4));
        assert_eq!(reader.read_bits(5)?, BitSequence::new(0b10110, 5));
        assert_eq!(reader.read_bits(8)?, BitSequence::new(0b01011111, 8));
        assert_eq!(
            reader.read_bits(2).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        Ok(())
    }

    #[test]
    fn borrow_reader_from_boundary() -> io::Result<()> {
        let data: &[u8] = &[0b01100011, 0b11011011, 0b10101111];
        let mut reader = BitReader::new(data);
        assert_eq!(reader.read_bits(3)?, BitSequence::new(0b011, 3));
        assert_eq!(reader.borrow_reader_from_boundary().read_u8()?, 0b11011011);
        assert_eq!(reader.read_bits(8)?, BitSequence::new(0b10101111, 8));
        Ok(())
    }
}
