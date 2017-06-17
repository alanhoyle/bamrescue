# bamrescue [![](https://img.shields.io/crates/v/bamrescue.svg)](https://crates.io/crates/bamrescue) [![License](http://img.shields.io/badge/license-ISC-blue.svg)](/LICENSE) [![Build status](https://travis-ci.org/Arkanosis/bamrescue.svg?branch=master)](https://travis-ci.org/Arkanosis/bamrescue)

**bamrescue** is a command line utility to check Binary Sequence
Alignment / Map (BAM) files for corruption and rescue as much data
as possible from them in the event they happen to be corrupted.

## How it works

A BAM file is a BGZF file ([specification](https://samtools.github.io/hts-specs/SAMv1.pdf)),
and as such is composed of a series of concatenated RFC1592-compliant gzip
blocks ([specification](https://tools.ietf.org/html/rfc1952)).

Each gzip block contains at most 64 KiB of data, including a CRC32 checksum of
the uncompressed data which is used to check its integrity.

Additionally, since gzip blocks start with a gzip identifier (ie. 0x1f8b), a
fixed gzip method (ie. 0x8) and fixed gzip flags (ie. 0x4), and bgzf blocks
include both a bgzf identifier (ie. 0x4243), a fixed extra subfield length
(ie. 0x2) and their own compressed size, it is possible to skip over corrupted
blocks (at most 64 KiB) to the next non-corrupted block with limited complexity
and acceptable reliability.

This property is used to rescue data from corrupted BAM files by keeping only
their non-corrupted blocks, hopefully rescuing most reads.

## Compilation

Run `cargo build --release` in your working copy.

## Installation

Copy the `bamrescue` binary wherever you want.

## Usage

```console
Usage: bamrescue check [--quiet] <bamfile>
       bamrescue repair <bamfile> <output>
       bamrescue -h | --help
       bamrescue --version

Commands:
    check        Check BAM file for corruption.
    repair       Keep only non-corrupted blocks of BAM file.

Arguments:
    bamfile      BAM file to check or repair.
    output       Repaired BAM file.

Options:
    -h, --help   Show this screen.
	-q, --quiet  Do not output statistics, stop at first error.
    --version    Show version.
```

## Contributing and reporting bugs

Contributions are welcome through [GitHub pull requests](https://github.com/Arkanosis/bamrescue/pulls).

Please report bugs and feature requests on [GitHub issues](https://github.com/Arkanosis/bamrescue/issues).

## License

bamrescue is copyright (C) 2017 Jérémie Roquet <jroquet@arkanosis.net> and
licensed under the ISC license.
