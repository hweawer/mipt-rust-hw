use std::io::BufRead;

use anyhow::{anyhow, Result};

use crate::bit_reader::{BitReader, BitSequence};
use crate::CompressionType::{DynamicTree, FixedTree, Reserved, Uncompressed};

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct BlockHeader {
    pub is_final: bool,
    pub compression_type: CompressionType,
}

#[derive(Debug)]
pub enum CompressionType {
    Uncompressed = 0,
    FixedTree = 1,
    DynamicTree = 2,
    Reserved = 3,
}

////////////////////////////////////////////////////////////////////////////////

pub struct DeflateReader<T> {
    bit_reader: BitReader<T>,
}

impl<T: BufRead> DeflateReader<T> {
    pub fn new(bit_reader: BitReader<T>) -> Self {
        Self { bit_reader }
    }

    pub fn next_block(&mut self) -> Option<Result<(BlockHeader, &mut BitReader<T>)>> {
        Some(self.inner())
    }

    fn inner(&mut self) -> Result<(BlockHeader, &mut BitReader<T>)> {
        let final_bit = self.bit_reader.read_bits(1)?;
        let is_final = final_bit.bits() == 1;
        let compression_bits = self.bit_reader.read_bits(2)?;
        let compression_type = match compression_bits.bits() {
            0 => Uncompressed,
            1 => FixedTree,
            2 => DynamicTree,
            _ => Reserved,
        };
        Ok((
            BlockHeader {
                is_final,
                compression_type,
            },
            &mut self.bit_reader,
        ))
    }
}
