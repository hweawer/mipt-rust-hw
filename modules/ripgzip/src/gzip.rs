use std::io::{BufRead};

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};

use crc::{crc32, Hasher32};

////////////////////////////////////////////////////////////////////////////////

const ID1: u8 = 0x1f;
const ID2: u8 = 0x8b;

const CM_DEFLATE: u8 = 8;

const FTEXT_OFFSET: u8 = 0;
const FHCRC_OFFSET: u8 = 1;
const FEXTRA_OFFSET: u8 = 2;
const FNAME_OFFSET: u8 = 3;
const FCOMMENT_OFFSET: u8 = 4;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MemberHeader {
    pub compression_method: CompressionMethod,
    pub modification_time: u32,
    pub extra: Option<Vec<u8>>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub extra_flags: u8,
    pub os: u8,
    pub has_crc: bool,
    pub is_text: bool,
}

impl MemberHeader {
    pub fn crc16(&self) -> u16 {
        let mut digest = crc32::Digest::new(crc32::IEEE);

        digest.write(&[ID1, ID2, self.compression_method.into(), self.flags().0]);
        digest.write(&self.modification_time.to_le_bytes());
        digest.write(&[self.extra_flags, self.os]);

        if let Some(extra) = &self.extra {
            digest.write(&(extra.len() as u16).to_le_bytes());
            digest.write(extra);
        }

        if let Some(name) = &self.name {
            digest.write(name.as_bytes());
            digest.write(&[0]);
        }

        if let Some(comment) = &self.comment {
            digest.write(comment.as_bytes());
            digest.write(&[0]);
        }

        (digest.sum32() & 0xffff) as u16
    }

    pub fn flags(&self) -> MemberFlags {
        let mut flags = MemberFlags(0);
        flags.set_is_text(self.is_text);
        flags.set_has_crc(self.has_crc);
        flags.set_has_extra(self.extra.is_some());
        flags.set_has_name(self.name.is_some());
        flags.set_has_comment(self.comment.is_some());
        flags
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub enum CompressionMethod {
    Deflate,
    Unknown(u8),
}

impl From<u8> for CompressionMethod {
    fn from(value: u8) -> Self {
        match value {
            CM_DEFLATE => Self::Deflate,
            x => Self::Unknown(x),
        }
    }
}

impl From<CompressionMethod> for u8 {
    fn from(method: CompressionMethod) -> u8 {
        match method {
            CompressionMethod::Deflate => CM_DEFLATE,
            CompressionMethod::Unknown(x) => x,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MemberFlags(u8);

#[allow(unused)]
impl MemberFlags {
    fn bit(&self, n: u8) -> bool {
        (self.0 >> n) & 1 != 0
    }

    fn set_bit(&mut self, n: u8, value: bool) {
        if value {
            self.0 |= 1 << n;
        } else {
            self.0 &= !(1 << n);
        }
    }

    pub fn is_text(&self) -> bool {
        self.bit(FTEXT_OFFSET)
    }

    pub fn set_is_text(&mut self, value: bool) {
        self.set_bit(FTEXT_OFFSET, value)
    }

    pub fn has_crc(&self) -> bool {
        self.bit(FHCRC_OFFSET)
    }

    pub fn set_has_crc(&mut self, value: bool) {
        self.set_bit(FHCRC_OFFSET, value)
    }

    pub fn has_extra(&self) -> bool {
        self.bit(FEXTRA_OFFSET)
    }

    pub fn set_has_extra(&mut self, value: bool) {
        self.set_bit(FEXTRA_OFFSET, value)
    }

    pub fn has_name(&self) -> bool {
        self.bit(FNAME_OFFSET)
    }

    pub fn set_has_name(&mut self, value: bool) {
        self.set_bit(FNAME_OFFSET, value)
    }

    pub fn has_comment(&self) -> bool {
        self.bit(FCOMMENT_OFFSET)
    }

    pub fn set_has_comment(&mut self, value: bool) {
        self.set_bit(FCOMMENT_OFFSET, value)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct MemberFooter {
    pub data_crc32: u32,
    pub data_size: u32,
}

////////////////////////////////////////////////////////////////////////////////

pub struct GzipReader<T> {
    reader: T,
}

impl<T: BufRead> GzipReader<T> {
    pub fn new(reader: T) -> Self {
        Self { reader }
    }

    pub fn next_member(mut self) -> Option<Result<(MemberHeader, MemberReader<T>)>> {
        match self.reader.has_data_left() {
            Ok(false) => return None,
            Err(x) => return Some(Err(anyhow!(x))),
            _ => (),
        }
        Some(self.inner())
    }

    fn inner(mut self) -> Result<(MemberHeader, MemberReader<T>)> {
        let mut byte = self.reader.read_u8()?;
        if byte != ID1 {
            return Err(anyhow!("wrong id values"));
        }
        byte = self.reader.read_u8()?;
        if byte != ID2 {
            return Err(anyhow!("wrong id values"));
        }
        byte = self.reader.read_u8()?;
        let compression_method = CompressionMethod::from(byte);
        match compression_method {
            CompressionMethod::Unknown(_) => {
                return Err(anyhow!("unsupported compression method"));
            }
            CompressionMethod::Deflate => {}
        }
        byte = self.reader.read_u8()?;

        let is_text = byte >> FTEXT_OFFSET & 1 == 1;
        let has_crc = byte >> FHCRC_OFFSET & 1 == 1;
        let is_fextra = byte >> FEXTRA_OFFSET & 1 == 1;
        let is_fname = byte >> FNAME_OFFSET & 1 == 1;
        let is_fcomment = byte >> FCOMMENT_OFFSET & 1 == 1;

        let modification_time = self.reader.read_u32::<LittleEndian>()?;
        let extra_flags = self.reader.read_u8()?;
        let os = self.reader.read_u8()?;
        let mut extra = None;
        if is_fextra {
            let xlen = self.reader.read_u16::<LittleEndian>()?;
            let mut temp = Vec::with_capacity(xlen as usize);
            for _ in 0..xlen {
                let byte = self.reader.read_u8()?;
                temp.push(byte);
            }
            extra = Some(temp);
        }
        let mut name = None;
        if is_fname {
            let mut buffer = Vec::new();
            let mut byte = self.reader.read_u8()?;
            while byte != 0 {
                buffer.push(byte);
                byte = self.reader.read_u8()?;
            }
            name = Some(String::from_utf8(buffer)?);
        }
        let mut comment = None;
        if is_fcomment {
            let mut buffer = Vec::new();
            let mut byte = self.reader.read_u8()?;
            while byte != 0 {
                buffer.push(byte);
                byte = self.reader.read_u8()?;
            }
            comment = Some(String::from_utf8(buffer)?);
        }

        let member_header = MemberHeader {
            compression_method,
            modification_time,
            extra,
            name,
            comment,
            extra_flags,
            os,
            has_crc,
            is_text,
        };

        if has_crc {
            let crc_16 = self.reader.read_u16::<LittleEndian>()?;
            if member_header.crc16() != crc_16 {
                return Err(anyhow!("header crc16 check failed"));
            }
        }
        Ok((member_header, MemberReader { inner: self.reader }))
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct MemberReader<T> {
    inner: T,
}

impl<T: BufRead> MemberReader<T> {
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn read_footer(mut self) -> Result<(MemberFooter, GzipReader<T>)> {
        let data_crc32 = self.inner.read_u32::<LittleEndian>()?;
        let data_size = self.inner.read_u32::<LittleEndian>()?;
        let gzip_reader = GzipReader::new(self.inner);
        let footer = MemberFooter {
            data_crc32,
            data_size,
        };
        Ok((footer, gzip_reader))
    }
}
