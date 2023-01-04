use std::fmt::Debug;
use std::{collections::HashMap, convert::TryFrom, io::BufRead};

use anyhow::{anyhow, Result};
#[cfg(not(test))]
use log::debug;

#[cfg(test)]
use std::println as debug;

use crate::bit_reader::{BitReader, BitSequence};
use crate::huffman_coding::TreeCodeToken::{CopyPrev, RepeatZero};
use crate::LitLenToken::{EndOfBlock, Length, Literal};

////////////////////////////////////////////////////////////////////////////////

pub fn decode_litlen_distance_trees<T: BufRead>(
    bit_reader: &mut BitReader<T>,
) -> Result<(HuffmanCoding<LitLenToken>, HuffmanCoding<DistanceToken>)> {
    let rows: usize = (bit_reader.read_bits(5)?.bits() + 257) as usize;
    debug!(
        "Number of literal/end-of-block/length codes (257-286) {}",
        rows
    );

    let distances_rows = (bit_reader.read_bits(5)?.bits() + 1) as usize;
    debug!("Number of distance codes (1-32) {}", distances_rows);

    let code_length_codes_number = bit_reader.read_bits(4)?.bits() + 4;
    debug!(
        "Number of code length codes used (4-19) {}",
        code_length_codes_number
    );

    let mut lengths = [0u8; 19];
    for len in lengths.iter_mut().take(code_length_codes_number as usize) {
        *len = bit_reader.read_bits(3)?.bits() as u8;
    }
    let mut swapped_lengths = [0u8; 19];
    swapped_lengths[16] = lengths[0];
    swapped_lengths[17] = lengths[1];
    swapped_lengths[18] = lengths[2];
    swapped_lengths[0] = lengths[3];
    swapped_lengths[8] = lengths[4];
    swapped_lengths[7] = lengths[5];
    swapped_lengths[9] = lengths[6];
    swapped_lengths[6] = lengths[7];
    swapped_lengths[10] = lengths[8];
    swapped_lengths[5] = lengths[9];
    swapped_lengths[11] = lengths[10];
    swapped_lengths[4] = lengths[11];
    swapped_lengths[12] = lengths[12];
    swapped_lengths[3] = lengths[13];
    swapped_lengths[13] = lengths[14];
    swapped_lengths[2] = lengths[15];
    swapped_lengths[14] = lengths[16];
    swapped_lengths[1] = lengths[17];
    swapped_lengths[15] = lengths[18];
    let huffman_coding = HuffmanCoding::<TreeCodeToken>::from_lengths(&swapped_lengths)?;
    let mut table = [0u8; 286];
    build_huffman_coding(&huffman_coding, &mut table, bit_reader, rows)?;
    let literal_huffman_coding = HuffmanCoding::<LitLenToken>::from_lengths(table.as_slice())?;
    let mut distances_table = [0u8; 30];
    build_huffman_coding(
        &huffman_coding,
        &mut distances_table,
        bit_reader,
        distances_rows,
    )?;
    let distance_huffman_coding =
        HuffmanCoding::<DistanceToken>::from_lengths(distances_table.as_slice())?;
    Ok((literal_huffman_coding, distance_huffman_coding))
}

fn build_huffman_coding<T: BufRead>(
    tree_code_coding: &HuffmanCoding<TreeCodeToken>,
    table: &mut [u8],
    reader: &mut BitReader<T>,
    rows: usize,
) -> Result<()> {
    let mut index: usize = 0;
    let mut prev: u8 = 0;
    while index < rows {
        match tree_code_coding.read_symbol(reader)? {
            TreeCodeToken::Length(len) => {
                table[index] = len;
                prev = len;
                index += 1;
            }
            CopyPrev => {
                let repeats = reader.read_bits(2)?.bits() + 3;
                for _ in 0..repeats {
                    table[index] = prev;
                    index += 1;
                }
            }
            RepeatZero { base, extra_bits } => {
                let number_of_zero_rows = (base + reader.read_bits(extra_bits)?.bits()) as usize;
                index += number_of_zero_rows;
            }
        }
    }
    Ok(())
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub enum TreeCodeToken {
    Length(u8),
    CopyPrev,
    RepeatZero { base: u16, extra_bits: u8 },
}

impl TryFrom<HuffmanCodeWord> for TreeCodeToken {
    type Error = anyhow::Error;

    fn try_from(value: HuffmanCodeWord) -> Result<Self> {
        if value.0 <= 15 {
            Ok(TreeCodeToken::Length(value.0 as u8))
        } else if value.0 == 16 {
            Ok(CopyPrev)
        } else if value.0 == 17 {
            Ok(RepeatZero {
                base: 3,
                extra_bits: 3,
            })
        } else if value.0 == 18 {
            Ok(RepeatZero {
                base: 11,
                extra_bits: 7,
            })
        } else {
            Err(anyhow!("Unknown value for TreeCodeToken: {}", value.0))
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub enum LitLenToken {
    Literal(u8),
    EndOfBlock,
    Length { base: u16, extra_bits: u8 },
}

impl TryFrom<HuffmanCodeWord> for LitLenToken {
    type Error = anyhow::Error;

    fn try_from(value: HuffmanCodeWord) -> Result<Self> {
        assert!(value.0 <= 285);
        if value.0 == 256 {
            Ok(EndOfBlock)
        } else if value.0 < 256 {
            Ok(Literal(value.0 as u8))
        } else if value.0 <= 264 {
            Ok(Length {
                base: 3 + (value.0 - 257),
                extra_bits: 0,
            })
        } else if value.0 <= 268 {
            let shift = (value.0 - 265) * 2;
            Ok(Length {
                base: 11 + shift,
                extra_bits: 1,
            })
        } else if value.0 <= 272 {
            let shift = (value.0 - 269) * 4;
            Ok(Length {
                base: 19 + shift,
                extra_bits: 2,
            })
        } else if value.0 <= 276 {
            let shift = (value.0 - 273) * 8;
            Ok(Length {
                base: 35 + shift,
                extra_bits: 3,
            })
        } else if value.0 <= 280 {
            let shift = (value.0 - 277) * 16;
            Ok(Length {
                base: 67 + shift,
                extra_bits: 4,
            })
        } else if value.0 <= 284 {
            let shift = (value.0 - 281) * 32;
            Ok(Length {
                base: 131 + shift,
                extra_bits: 5,
            })
        } else {
            Ok(Length {
                base: 258,
                extra_bits: 0,
            })
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug)]
pub struct DistanceToken {
    pub base: u16,
    pub extra_bits: u8,
}

impl TryFrom<HuffmanCodeWord> for DistanceToken {
    type Error = anyhow::Error;

    // todo: make a table on a compile time
    fn try_from(value: HuffmanCodeWord) -> Result<Self> {
        assert!(value.0 <= 29);
        if value.0 <= 3 {
            Ok(DistanceToken {
                base: value.0 + 1,
                extra_bits: 0,
            })
        } else if value.0 <= 5 {
            let shift = (value.0 - 4) * 2;
            Ok(DistanceToken {
                base: 5 + shift,
                extra_bits: 1,
            })
        } else if value.0 <= 7 {
            let shift = (value.0 - 6) * 4;
            Ok(DistanceToken {
                base: 9 + shift,
                extra_bits: 2,
            })
        } else if value.0 <= 9 {
            let shift = (value.0 - 8) * 8;
            Ok(DistanceToken {
                base: 17 + shift,
                extra_bits: 3,
            })
        } else if value.0 <= 11 {
            let shift = (value.0 - 10) * 16;
            Ok(DistanceToken {
                base: 33 + shift,
                extra_bits: 4,
            })
        } else if value.0 <= 13 {
            let shift = (value.0 - 12) * 32;
            Ok(DistanceToken {
                base: 65 + shift,
                extra_bits: 5,
            })
        } else if value.0 <= 15 {
            let shift = (value.0 - 14) * 64;
            Ok(DistanceToken {
                base: 129 + shift,
                extra_bits: 6,
            })
        } else if value.0 <= 17 {
            let shift = (value.0 - 16) * 128;
            Ok(DistanceToken {
                base: 257 + shift,
                extra_bits: 7,
            })
        } else if value.0 <= 19 {
            let shift = (value.0 - 18) * 256;
            Ok(DistanceToken {
                base: 513 + shift,
                extra_bits: 8,
            })
        } else if value.0 <= 21 {
            let shift = (value.0 - 20) * 512;
            Ok(DistanceToken {
                base: 1025 + shift,
                extra_bits: 9,
            })
        } else if value.0 <= 23 {
            let shift = (value.0 - 22) * 1024;
            Ok(DistanceToken {
                base: 2049 + shift,
                extra_bits: 10,
            })
        } else if value.0 <= 25 {
            let shift = (value.0 - 24) * 2048;
            Ok(DistanceToken {
                base: 4097 + shift,
                extra_bits: 11,
            })
        } else if value.0 <= 27 {
            let shift = (value.0 - 26) * 4096;
            Ok(DistanceToken {
                base: 8193 + shift,
                extra_bits: 12,
            })
        } else {
            let shift = (value.0 - 28) * 8192;
            Ok(DistanceToken {
                base: 16385 + shift,
                extra_bits: 13,
            })
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

const MAX_BITS: usize = 15;

pub struct HuffmanCodeWord(pub u16);

#[derive(Debug)]
pub struct HuffmanCoding<T> {
    map: HashMap<BitSequence, T>,
}

impl<T> HuffmanCoding<T>
where
    T: Copy + TryFrom<HuffmanCodeWord, Error = anyhow::Error> + Debug,
{
    pub fn decode_symbol(&self, seq: BitSequence) -> Option<T> {
        self.map.get(&seq).cloned()
    }

    pub fn read_symbol<U: BufRead>(&self, bit_reader: &mut BitReader<U>) -> Result<T> {
        let mut seq = bit_reader.read_bits(1)?;
        loop {
            if self.map.contains_key(&seq) {
                return Ok(*self.map.get(&seq).unwrap());
            }
            let temp = bit_reader.read_bits(1)?;
            seq = seq.concat(temp);
        }
    }

    pub fn from_lengths(code_lengths: &[u8]) -> Result<Self> {
        let mut bl_count: HashMap<u8, u16> = HashMap::new();
        for e in code_lengths {
            let key = *e;
            if key > 0 {
                let lower_border_for_len = *bl_count.entry(key).or_insert(0) + 1;
                bl_count.insert(key, lower_border_for_len);
            }
        }
        let mut next_code = [0u16; MAX_BITS + 1];
        let mut code = 0;
        for bits in 1..=MAX_BITS {
            code = (code + bl_count.get(&(bits as u8 - 1)).unwrap_or(&0)) << 1;
            next_code[bits] = code;
        }
        let mut result = HashMap::new();
        for (i, len) in code_lengths.iter().enumerate() {
            let len = *len as usize;
            if len > 0 {
                let seq = BitSequence::new(next_code[len], len as u8);
                let elem = T::try_from(HuffmanCodeWord(i as u16))?;
                result.insert(seq, elem);
                next_code[len as usize] += 1;
            }
        }
        Ok(Self { map: result })
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Value(u16);

    impl TryFrom<HuffmanCodeWord> for Value {
        type Error = anyhow::Error;

        fn try_from(x: HuffmanCodeWord) -> Result<Self> {
            Ok(Self(x.0))
        }
    }

    #[test]
    fn from_lengths() -> Result<()> {
        let code = HuffmanCoding::<Value>::from_lengths(&[2, 3, 4, 3, 3, 4, 2])?;

        assert_eq!(
            code.decode_symbol(BitSequence::new(0b00, 2)),
            Some(Value(0)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b100, 3)),
            Some(Value(1)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b1110, 4)),
            Some(Value(2)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b101, 3)),
            Some(Value(3)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b110, 3)),
            Some(Value(4)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b1111, 4)),
            Some(Value(5)),
        );
        assert_eq!(
            code.decode_symbol(BitSequence::new(0b01, 2)),
            Some(Value(6)),
        );

        assert_eq!(code.decode_symbol(BitSequence::new(0b0, 1)), None);
        assert_eq!(code.decode_symbol(BitSequence::new(0b10, 2)), None);
        assert_eq!(code.decode_symbol(BitSequence::new(0b111, 3)), None,);

        Ok(())
    }

    #[test]
    fn read_symbol() -> Result<()> {
        let code = HuffmanCoding::<Value>::from_lengths(&[2, 3, 4, 3, 3, 4, 2])?;
        let mut data: &[u8] = &[0b10111001, 0b11001010, 0b11101101];
        let mut reader = BitReader::new(&mut data);

        assert_eq!(code.read_symbol(&mut reader)?, Value(1));
        assert_eq!(code.read_symbol(&mut reader)?, Value(2));
        assert_eq!(code.read_symbol(&mut reader)?, Value(3));
        assert_eq!(code.read_symbol(&mut reader)?, Value(6));
        assert_eq!(code.read_symbol(&mut reader)?, Value(0));
        assert_eq!(code.read_symbol(&mut reader)?, Value(2));
        assert_eq!(code.read_symbol(&mut reader)?, Value(4));
        assert!(code.read_symbol(&mut reader).is_err());

        Ok(())
    }
}
