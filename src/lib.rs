extern crate byteorder;
extern crate crc;
extern crate inflate;
#[macro_use]
extern crate slog;

use byteorder::ReadBytesExt;

use crc::crc32::Hasher32;

use std::fs::File;
use std::io::BufReader;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::str;

const GZIP_IDENTIFIER: [u8; 2] = [0x1f, 0x8b];
const BGZF_IDENTIFIER: [u8; 2] = [0x42, 0x43];

const DEFLATE: u8 = 8;

const EXTRA: u8 = 1 << 2;

pub fn version() -> &'static str {
    return option_env!("CARGO_PKG_VERSION").unwrap_or("unknown");
}

enum BGZFBlockInformation {
    EOF,
    Size(u32)
}

fn check_block(reader: &mut BufReader<File>, blocks_count: &mut u64, logger: &slog::Logger) -> Result<BGZFBlockInformation, Error> {
    let mut gzip_identifier = [0u8; 2];
    let read_bytes = reader.read(&mut gzip_identifier)?;
    if read_bytes == 0 {
        return Ok(BGZFBlockInformation::EOF);
    }
    if read_bytes != 2 || gzip_identifier != GZIP_IDENTIFIER {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid bam file: gzip identitifer not found"));
    }
    *blocks_count += 1;
    debug!(logger, "Checking block {}", blocks_count);

    let mut compression_method = [0u8; 1];
    reader.read_exact(&mut compression_method)?;
    if compression_method[0] != DEFLATE {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid bam file: gzip compression method is not deflate"));
    }

    let mut flags = [0u8; 1];
    reader.read_exact(&mut flags)?;
    if flags[0] != EXTRA {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid bam file: unexpected gzip flags"));
    }

    let mut modification_time = [0u8; 4];
    reader.read_exact(&mut modification_time)?;

    let mut extra_flags = [0u8; 1];
    reader.read_exact(&mut extra_flags)?;

    let mut operating_system = [0u8; 1];
    reader.read_exact(&mut operating_system)?;

    let mut block_size = 0u16;

    let extra_field_length = reader.read_u16::<byteorder::LittleEndian>()?;
    debug!(logger, "\tExtra field length of {} bytes", extra_field_length);

    let mut remaining_extra_field_length = extra_field_length;
    while remaining_extra_field_length > 0 {
        let mut subfield_identifier = [0u8; 2];
        reader.read_exact(&mut subfield_identifier)?;

        let subfield_length = reader.read_u16::<byteorder::LittleEndian>()?;
        debug!(logger, "\t\tSubfield length of {} bytes", subfield_length);

        if subfield_identifier == BGZF_IDENTIFIER {
            debug!(logger, "\t\t\tSubfield is bgzf metadata");

            if subfield_length != 2 {
                return Err(Error::new(ErrorKind::InvalidData, "Invalid bam file: bgzf block size is not a 16 bits number"));
            }

            block_size = reader.read_u16::<byteorder::LittleEndian>()? + 1;
            debug!(logger, "\t\t\t\tbgzf block size is {} bytes", block_size);
        } else {
            reader.seek(SeekFrom::Current(subfield_length as i64))?;
        }

        remaining_extra_field_length -= 4 + subfield_length;
    }
    if block_size == 0 {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid bam file: bgzf block size not found in gzip extra field"));
    }

    let mut payload_digest = crc::crc32::Digest::new(crc::crc32::IEEE);
    let payload_size;
    {
        let mut deflated_bytes = vec![];
        let mut deflate_reader = reader.take((block_size - extra_field_length - 20u16) as u64);
        deflate_reader.read_to_end(&mut deflated_bytes)?;
        let inflated_bytes = match inflate::inflate_bytes(&deflated_bytes) {
            Ok(inflated_bytes) => inflated_bytes,
            Err(error) => return Err(Error::new(ErrorKind::InvalidData, format!("Invalid bam file: unable to inflate payload: {}", error))),
        };
        payload_digest.write(&inflated_bytes);
        payload_size = inflated_bytes.len();
    }

    let mut data_crc32 = [0u8; 4];
    reader.read_exact(&mut data_crc32)?;
    let payload_crc32 = payload_digest.sum32();

    if data_crc32[0] != ((payload_crc32 & 0xff) as u8) ||
       data_crc32[1] != (((payload_crc32 >> 8) & 0xff) as u8) ||
       data_crc32[2] != (((payload_crc32 >> 16) & 0xff) as u8) ||
       data_crc32[3] != (((payload_crc32 >> 24) & 0xff) as u8) {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid bam file: incorrect payload CRC32"));
    }

    let data_size = reader.read_u32::<byteorder::LittleEndian>()?;
    debug!(logger, "\tData size is {} bytes", data_size);
    if data_size as usize != payload_size {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid bam file: incorrect payload size"));
    }

    debug!(logger, "\tAt offset {}", reader.seek(SeekFrom::Current(0))?);

    return Ok(BGZFBlockInformation::Size(data_size));
}

pub fn check(bamfile: &str, logger: &slog::Logger) -> Result<(), Error> {
    info!(logger, "Checking integrity of {}…", bamfile);

    let mut reader = BufReader::new(File::open(bamfile)?);

    let mut blocks_count = 0u64;
    let mut data_size = 0u32;
    loop {
        data_size = match check_block(&mut reader, &mut blocks_count, &logger)? {
            BGZFBlockInformation::EOF => if data_size == 0u32 { break } else { return Err(Error::new(ErrorKind::InvalidData, "Invalid bam file: unexpected end of file while last bgzf block was not empty")); },
            BGZFBlockInformation::Size(data_size) => data_size,
        };
    }

    println!("bam file statistics:");
    println!("{: >7} bgzf blocks found", blocks_count);
    println!("{: >7} corrupted blocks found", 0);
    Ok(())
}

pub fn repair(bamfile: &str, output: &str, logger: &slog::Logger) -> Result<(), Error> {
    info!(logger, "Repairing {} and writing output to {}…", bamfile, output);

    error!(logger, "bamrescue::repair() is not yet implemented");
    unimplemented!();
}
