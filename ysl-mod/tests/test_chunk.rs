mod common;
use ysl_mod::package;

#[test]
fn parse_empty() {
    let input: Vec<u8> = common::ChunkBuilder::new().remove_len().build();
    let rej = package::read::parse_chunk(&input).unwrap_err();
    assert_eq!(
        rej,
        package::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 0
        },
    )
}

#[test]
fn parse_not_enough_data_for_header_fourcc() {
    let input: Vec<u8> = common::ChunkBuilder::new().id(b"ME").remove_len().build();
    let rej = package::read::parse_chunk(&input).unwrap_err();
    assert_eq!(
        rej,
        package::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 2
        },
    )
}

#[test]
fn parse_not_enough_data_for_header_len() {
    let input: Vec<u8> = common::ChunkBuilder::new()
        .id(common::ID_META)
        .arbitrary_len(&[0x00, 0x01, 0x02])
        .build();
    let rej = package::read::parse_chunk(&input).unwrap_err();
    assert_eq!(
        rej,
        package::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 7
        },
    )
}

#[test]
fn parse_empty_payload() {
    let input: Vec<u8> = common::ChunkBuilder::new().id(common::ID_RHAI).build();
    let (parsed_chunk, rest) = package::read::parse_chunk(&input).expect("valid operation");
    assert_eq!(
        parsed_chunk,
        package::Chunk {
            four_cc: *b"RHAI",
            payload: &[]
        }
    );
    assert_eq!(rest, &[]);
}

#[test]
fn parse_not_enough_data_for_payload() {
    let input: Vec<u8> = common::ChunkBuilder::new()
        .id(common::ID_META)
        .len(3)
        .payload(&[0xA0])
        .build();
    let rej = package::read::parse_chunk(&input).unwrap_err();
    assert_eq!(
        rej,
        package::ChunkRejection::NotEnoughDataPayload {
            declared: 3,
            available: 1
        }
    )
}

#[test]
fn parse_valid_chunk() {
    let input: Vec<u8> = common::ChunkBuilder::new()
        .id(common::ID_META)
        .len(3)
        .payload(&[0xA0, 0xA1, 0xA2])
        .build();
    let (parsed_chunk, rest) = package::read::parse_chunk(&input).expect("valid operation");
    assert_eq!(
        parsed_chunk,
        package::Chunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }
    );
    assert_eq!(rest, &[]);
}

#[test]
fn parse_valid_chunk_extra_byte() {
    let input: Vec<u8> = common::ChunkBuilder::new()
        .id(common::ID_META)
        .len(3)
        .payload(&[0xA0, 0xA1, 0xA2, 0xA3])
        .build();
    let (parsed_chunk, rest) = package::read::parse_chunk(&input).expect("valid operation");
    assert_eq!(
        parsed_chunk,
        package::Chunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }
    );
    assert_eq!(rest, &[0xA3]);
}

#[test]
fn parse_len_max() {
    let input: Vec<u8> = common::ChunkBuilder::new()
        .id(common::ID_META)
        .len(u32::MAX)
        .payload(&[0xA0, 0xA1, 0xA2])
        .build();
    let rej = package::read::parse_chunk(&input).unwrap_err();
    assert_eq!(
        rej,
        package::ChunkRejection::NotEnoughDataPayload {
            declared: u32::MAX,
            available: 3
        }
    )
}

#[test]
fn parse_consecutive_chunks() {
    let mut input: Vec<u8> = vec![];
    input.extend_from_slice(
        &common::ChunkBuilder::new()
            .id(common::ID_META)
            .payload(&[0xA0, 0xA1, 0xA2])
            .build(),
    );
    input.extend_from_slice(
        &common::ChunkBuilder::new()
            .id(common::ID_RHAI)
            .payload(&[0xB0, 0xB1, 0xB2, 0xB3])
            .build(),
    );

    let (first_parsed_chunk, first_rest) =
        package::read::parse_chunk(&input).expect("valid operation");
    assert_eq!(
        first_parsed_chunk,
        package::Chunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }
    );
    let (second_parsed_chunk, second_rest) =
        package::read::parse_chunk(first_rest).expect("valid operation");
    assert_eq!(
        second_parsed_chunk,
        package::Chunk {
            four_cc: *b"RHAI",
            payload: &[0xB0, 0xB1, 0xB2, 0xB3]
        }
    );
    assert_eq!(second_rest, &[]);
}

#[test]
fn parse_iterator_stop() {
    let mut input: Vec<u8> = vec![];
    input.extend_from_slice(
        &common::ChunkBuilder::new()
            .id(common::ID_META)
            .payload(&[0xA0, 0xA1, 0xA2])
            .build(),
    );
    input.extend_from_slice(
        &common::ChunkBuilder::new()
            .id(common::ID_META)
            .remove_len()
            .build(),
    );
    let mut iter = package::read::ChunkIter::from(&input[..]);

    assert_eq!(
        iter.next(),
        Some(Ok(package::Chunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }))
    );
    assert_eq!(
        iter.next(),
        Some(Err(package::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 4
        }))
    );
    assert_eq!(iter.next(), None);
}
