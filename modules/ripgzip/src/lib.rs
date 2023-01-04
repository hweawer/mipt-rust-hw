#![feature(buf_read_has_data_left)]
#![forbid(unsafe_code)]

extern crate core;

use std::io;
use std::io::{BufRead, Read, Write};

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::huffman_coding::TreeCodeToken;
use bit_reader::BitReader;
use deflate::{CompressionType, DeflateReader};
use gzip::GzipReader;
use huffman_coding::{decode_litlen_distance_trees, LitLenToken};
use log::*;
use tracking_writer::TrackingWriter;

#[cfg(not(test))]
use log::debug;
use log::*;

#[cfg(test)]
use std::println as debug;

mod bit_reader;
mod deflate;
mod gzip;
mod huffman_coding;
mod tracking_writer;

////////////////////////////////////////////////////////////////////////////////

pub fn compress<R: BufRead, W: Write>(_input: R, _output: W) -> Result<()> {
    // todo: tba in future
    unimplemented!()
}

////////////////////////////////////////////////////////////////////////////////

pub fn decompress<R: BufRead, W: Write>(input: R, mut output: W) -> Result<()> {
    let test: &[u8] = &[
        0x1fu8, 0x8bu8, 0x08u8, 0x00u8, 0x00u8, 0x00u8, 0x00u8, 0x00u8, 0x00u8, 0x03u8, 0x1du8,
        0xc6u8, 0x49u8, 0x01u8, 0x00u8, 0x00u8, 0x10u8, 0x40u8, 0xc0u8, 0xacu8, 0xa3u8, 0x7fu8,
        0x88u8, 0x3du8, 0x3cu8, 0x20u8, 0x2au8, 0x97u8, 0x9du8, 0x37u8, 0x5eu8, 0x1du8, 0x0cu8,
        0x6eu8, 0x29u8, 0x34u8, 0x94u8, 0x23u8, 0x00u8, 0x00u8, 0x00u8,
    ];
    let mut gzip_reader = GzipReader::new(input);
    let mut tracking_writer = TrackingWriter::new(output);
    while let Some(member) = gzip_reader.next_member() {
        let (_, mut y) = member?;
        let mut reader = DeflateReader::new(BitReader::new(y.inner_mut()));
        while let Some(block) = reader.next_block() {
            let (header, mut r) = block?;
            match header.compression_type {
                CompressionType::Uncompressed => {
                    let _ = r.borrow_reader_from_boundary();
                    let len = r.read_bits(16)?.bits();
                    let nlen = r.read_bits(16)?.bits();
                    if len != !nlen {
                        return Err(anyhow!("nlen check failed"));
                    }
                    for _ in 0..len {
                        let _ = tracking_writer.write(&[r.read_bits(8)?.bits() as u8])?;
                    }
                }
                CompressionType::DynamicTree => {
                    let (litlen_coding, distance_coding) = decode_litlen_distance_trees(r)?;
                    while let token = litlen_coding.read_symbol(r)? {
                        match token {
                            LitLenToken::Literal(lit) => {
                                let _ = tracking_writer.write(&[lit])?;
                            }
                            LitLenToken::EndOfBlock => break,
                            LitLenToken::Length { base, extra_bits } => {
                                let len = (base + r.read_bits(extra_bits)?.bits()) as usize;
                                let distance_token = distance_coding.read_symbol(r)?;
                                let distance = (distance_token.base
                                    + r.read_bits(distance_token.extra_bits)?.bits())
                                    as usize;
                                tracking_writer.write_previous(distance, len)?;
                            }
                        }
                    }
                }
                _ => {
                    return Err(anyhow!("unsupported block type"));
                }
            }
            if header.is_final {
                break;
            }
        }
        let (footer, new_gzip_reader) = y.read_footer()?;
        if footer.data_size as usize != tracking_writer.byte_count() {
            return Err(anyhow!("length check failed"));
        }
        if footer.data_crc32 != tracking_writer.crc32() {
            return Err(anyhow!("crc32 check failed"));
        }
        gzip_reader = new_gzip_reader;
        tracking_writer.flush()?;
    }
    Ok(())
}
