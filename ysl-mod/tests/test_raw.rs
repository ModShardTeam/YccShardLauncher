mod common;
use ysl_mod::raw;

#[test]
fn parse_empty() {
    let input: Vec<u8> = common::create_raw_chunk(common::ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    let error_chunk = raw::read::parse_chunk(&input[..0]).unwrap_err();
    assert_eq!(
        error_chunk,
        raw::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 0
        },
    )
}

#[test]
fn parse_not_enough_data_for_header_fourcc() {
    let input: Vec<u8> = common::create_raw_chunk(common::ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    let error_chunk = raw::read::parse_chunk(&input[..2]).unwrap_err();
    assert_eq!(
        error_chunk,
        raw::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 2
        },
    )
}

#[test]
fn parse_not_enough_data_for_header_len() {
    let input: Vec<u8> = common::create_raw_chunk(common::ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    let error_chunk = raw::read::parse_chunk(&input[..7]).unwrap_err();
    assert_eq!(
        error_chunk,
        raw::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 7
        },
    )
}

#[test]
fn parse_empty_payload() {
    let input: Vec<u8> = common::create_raw_chunk(common::ID_RHAI, 0, &[]);
    let (parsed_chunk, rest) = raw::read::parse_chunk(&input).expect("valid operation");
    assert_eq!(
        parsed_chunk,
        raw::Chunk {
            four_cc: *b"RHAI",
            payload: &[]
        }
    );
    assert_eq!(rest, &[]);
}

#[test]
fn parse_not_enough_data_for_payload() {
    let input: Vec<u8> = common::create_raw_chunk(common::ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    let error_chunk = raw::read::parse_chunk(&input[..9]).unwrap_err();
    assert_eq!(
        error_chunk,
        raw::ChunkRejection::NotEnoughDataPayload {
            declared: 3,
            available: 1
        }
    )
}

#[test]
fn parse_valid_chunk() {
    let input: Vec<u8> = common::create_raw_chunk(common::ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    let (parsed_chunk, rest) = raw::read::parse_chunk(&input).expect("valid operation");
    assert_eq!(
        parsed_chunk,
        raw::Chunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }
    );
    assert_eq!(rest, &[]);
}

#[test]
fn parse_valid_chunk_extra_byte() {
    let mut input: Vec<u8> = common::create_raw_chunk(common::ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    input.extend_from_slice(&[0xA3]);
    let (parsed_chunk, rest) = raw::read::parse_chunk(&input).expect("valid operation");
    assert_eq!(
        parsed_chunk,
        raw::Chunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }
    );
    assert_eq!(rest, &[0xA3]);
}

#[test]
fn parse_len_max() {
    let input: Vec<u8> = common::create_raw_chunk(common::ID_META, u32::MAX, &[0xA0, 0xA1, 0xA2]);
    let error_chunk = raw::read::parse_chunk(&input).unwrap_err();
    assert_eq!(
        error_chunk,
        raw::ChunkRejection::NotEnoughDataPayload {
            declared: u32::MAX,
            available: 3
        }
    )
}

#[test]
fn parse_consecutive_chunks() {
    let mut input: Vec<u8> = common::create_raw_chunk(common::ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    input.extend_from_slice(&common::create_raw_chunk(
        common::ID_RHAI,
        4,
        &[0xB0, 0xB1, 0xB2, 0xB3],
    ));

    let (first_parsed_chunk, first_rest) = raw::read::parse_chunk(&input).expect("valid operation");
    assert_eq!(
        first_parsed_chunk,
        raw::Chunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }
    );
    let (second_parsed_chunk, second_rest) =
        raw::read::parse_chunk(first_rest).expect("valid operation");
    assert_eq!(
        second_parsed_chunk,
        raw::Chunk {
            four_cc: *b"RHAI",
            payload: &[0xB0, 0xB1, 0xB2, 0xB3]
        }
    );
    assert_eq!(second_rest, &[]);
}

#[test]
fn parse_iterator_stop() {
    let mut broken_stream = common::create_raw_chunk(common::ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    broken_stream.extend_from_slice(b"META");
    let mut iter = raw::read::ChunkIter::from(&broken_stream[..]);

    assert_eq!(
        iter.next(),
        Some(Ok(raw::Chunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }))
    );
    assert_eq!(
        iter.next(),
        Some(Err(raw::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 4
        }))
    );
    assert_eq!(iter.next(), None);
}

#[test]
fn parse_valid_file() {
    let chunks: Vec<u8> = common::sample_raw_body();
    let mut file = common::create_raw_header(b"YFF\0", 1, 0, chunks.len() as u32);
    file.extend_from_slice(&chunks);

    let f = raw::read::parse_file(&file).expect("valid operation");
    assert_eq!(
        f,
        raw::File {
            format_version: 1,
            flags: 0,
            chunks: vec![
                raw::Chunk {
                    four_cc: *common::ID_META,
                    payload: &[0xA0, 0xA1, 0xA2]
                },
                raw::Chunk {
                    four_cc: *common::ID_RHAI,
                    payload: &[0xB0, 0xB1, 0xB2, 0xB3]
                },
                raw::Chunk {
                    four_cc: *common::ID_RHAI,
                    payload: &[0xC0, 0xC1, 0xC2, 0xC3]
                },
                raw::Chunk {
                    four_cc: *common::ID_RHAI,
                    payload: &[0xD0, 0xD1, 0xD2, 0xD3]
                },
                raw::Chunk {
                    four_cc: *common::ID_RHAI,
                    payload: &[0x00, 0x01, 0x02, 0x03]
                }
            ],
        }
    );
}

#[test]
fn parse_bad_magic_file() {
    let chunks: Vec<u8> = common::sample_raw_body();
    let mut file = common::create_raw_header(b"BAD\0", 1, 0, chunks.len() as u32);
    file.extend_from_slice(&chunks);

    let rej = raw::read::parse_file(&file).unwrap_err();
    assert_eq!(rej, raw::RawRejection::BadMagic { found: *b"BAD\0" });
}

#[test]
fn parse_null_version_file() {
    let chunks: Vec<u8> = common::sample_raw_body();
    let mut file = common::create_raw_header(b"YFF\0", 0, 0, chunks.len() as u32);
    file.extend_from_slice(&chunks);

    let rej = raw::read::parse_file(&file).unwrap_err();
    assert_eq!(rej, raw::RawRejection::UnsupportedVersion { found: 0 });
}

#[test]
fn parse_version_from_the_future_file() {
    let chunks: Vec<u8> = common::sample_raw_body();
    let mut file = common::create_raw_header(b"YFF\0", u16::MAX, 0, chunks.len() as u32);
    file.extend_from_slice(&chunks);

    let rej = raw::read::parse_file(&file).unwrap_err();
    assert_eq!(
        rej,
        raw::RawRejection::UnsupportedVersion { found: u16::MAX }
    );
}

#[test]
fn parse_unknown_low_byte_flag_file() {
    let chunks: Vec<u8> = common::sample_raw_body();
    let mut file =
        common::create_raw_header(b"YFF\0", 1, 0b0000_0000_1100_0000, chunks.len() as u32);
    file.extend_from_slice(&chunks);

    let rej = raw::read::parse_file(&file).unwrap_err();
    assert_eq!(
        rej,
        raw::RawRejection::UnknownBits {
            found: 0b0000_0000_1100_0000
        }
    );
}

#[test]
fn parse_unknown_high_byte_flag_file() {
    let chunks: Vec<u8> = common::sample_raw_body();
    let mut file =
        common::create_raw_header(b"YFF\0", 1, 0b0000_1101_0000_0000, chunks.len() as u32);
    file.extend_from_slice(&chunks);

    let rej = raw::read::parse_file(&file).unwrap_err();
    assert_eq!(
        rej,
        raw::RawRejection::UnknownBits {
            found: 0b0000_1101_0000_0000
        }
    );
}

#[test]
fn parse_too_long_len_file() {
    let chunks: Vec<u8> = common::sample_raw_body();
    let mut file = common::create_raw_header(b"YFF\0", 1, 0, chunks.len() as u32 + 10);
    file.extend_from_slice(&chunks);

    let rej = raw::read::parse_file(&file).unwrap_err();
    assert_eq!(
        rej,
        raw::RawRejection::InvalidSizeFile {
            declared: 69,
            available: 59
        }
    );
}

#[test]
fn parse_too_small_len_file() {
    let chunks: Vec<u8> = common::sample_raw_body();
    let mut file = common::create_raw_header(b"YFF\0", 1, 0, chunks.len() as u32 - 10);
    file.extend_from_slice(&chunks);

    let rej = raw::read::parse_file(&file).unwrap_err();
    assert_eq!(
        rej,
        raw::RawRejection::InvalidSizeFile {
            declared: 49,
            available: 59
        }
    );
}

#[test]
fn parse_valid_empty_chunks_file() {
    let file = common::create_raw_header(b"YFF\0", 1, 0, 0);

    let f = raw::read::parse_file(&file).expect("valid operation");
    assert_eq!(
        f,
        raw::File {
            format_version: 1,
            flags: 0,
            chunks: vec![],
        }
    );
}

#[test]
fn parse_unknown_chunk_id_file() {
    let mut chunks: Vec<u8> = vec![];
    chunks.extend_from_slice(&common::create_raw_chunk(b"META", 3, &[0xA0, 0xA1, 0xA2]));
    chunks.extend_from_slice(&common::create_raw_chunk(b"NOT\0", 3, &[0xB0, 0xB1, 0xB2]));
    chunks.extend_from_slice(&common::create_raw_chunk(b"META", 3, &[0xC0, 0xC1, 0xC2]));
    let mut file = common::create_raw_header(b"YFF\0", 1, 0, chunks.len() as u32);
    file.extend_from_slice(&chunks);

    let f = raw::read::parse_file(&file).expect("valid operation");
    assert_eq!(
        f,
        raw::File {
            format_version: 1,
            flags: 0,
            chunks: vec![
                raw::Chunk {
                    four_cc: *b"META",
                    payload: &[0xA0, 0xA1, 0xA2]
                },
                raw::Chunk {
                    four_cc: *b"NOT\0",
                    payload: &[0xB0, 0xB1, 0xB2]
                },
                raw::Chunk {
                    four_cc: *b"META",
                    payload: &[0xC0, 0xC1, 0xC2]
                }
            ],
        }
    );
}

#[test]
fn parse_partial_header_chunk_file() {
    let mut chunks: Vec<u8> = vec![];
    chunks.extend_from_slice(&common::create_raw_chunk(b"META", 3, &[0xA0, 0xA1, 0xA2]));
    chunks.extend_from_slice(b"META");
    let mut file = common::create_raw_header(b"YFF\0", 1, 0, chunks.len() as u32);
    file.extend_from_slice(&chunks);

    let rej = raw::read::parse_file(&file).unwrap_err();
    assert_eq!(
        rej,
        raw::RawRejection::InvalidChunk(raw::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 4
        })
    );
}

#[test]
fn parse_overrun_chunk_file() {
    let mut chunks: Vec<u8> = vec![];
    chunks.extend_from_slice(&common::create_raw_chunk(b"META", 3, &[0xA0, 0xA1, 0xA2]));
    chunks.extend_from_slice(&common::create_raw_chunk(b"META", 10, &[0xB0, 0xB1, 0xB2]));
    let mut file = common::create_raw_header(b"YFF\0", 1, 0, chunks.len() as u32);
    file.extend_from_slice(&chunks);

    let rej = raw::read::parse_file(&file).unwrap_err();
    assert_eq!(
        rej,
        raw::RawRejection::InvalidChunk(raw::ChunkRejection::NotEnoughDataPayload {
            declared: 10,
            available: 3
        })
    );
}
