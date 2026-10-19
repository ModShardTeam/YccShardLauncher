const ID_META: &[u8; 4] = b"META";
const ID_RHAI: &[u8; 4] = b"RHAI";

fn create_byte_array(id: &[u8; 4], len: u32, payload: &[u8]) -> Vec<u8> {
    let mut array: Vec<u8> = vec![];
    array.extend_from_slice(id);
    array.extend_from_slice(&len.to_le_bytes());
    array.extend_from_slice(payload);
    array
}

#[test]
fn parse_empty() {
    let input: Vec<u8> = create_byte_array(ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    let error_chunk = ysl_mod::parse_to_chunk(&input[..0]).unwrap_err();
    assert_eq!(
        error_chunk,
        ysl_mod::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 0
        },
    )
}

#[test]
fn parse_not_enough_data_for_header_fourcc() {
    let input: Vec<u8> = create_byte_array(ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    let error_chunk = ysl_mod::parse_to_chunk(&input[..2]).unwrap_err();
    assert_eq!(
        error_chunk,
        ysl_mod::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 2
        },
    )
}

#[test]
fn parse_not_enough_data_for_header_len() {
    let input: Vec<u8> = create_byte_array(ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    let error_chunk = ysl_mod::parse_to_chunk(&input[..7]).unwrap_err();
    assert_eq!(
        error_chunk,
        ysl_mod::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 7
        },
    )
}

#[test]
fn parse_empty_payload() {
    let input: Vec<u8> = create_byte_array(ID_RHAI, 0, &[]);
    let (parsed_chunk, rest) = ysl_mod::parse_to_chunk(&input).expect("valid operation");
    assert_eq!(
        parsed_chunk,
        ysl_mod::RawChunk {
            four_cc: *b"RHAI",
            payload: &[]
        }
    );
    assert_eq!(rest, &[]);
}

#[test]
fn parse_not_enough_data_for_payload() {
    let input: Vec<u8> = create_byte_array(ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    let error_chunk = ysl_mod::parse_to_chunk(&input[..9]).unwrap_err();
    assert_eq!(
        error_chunk,
        ysl_mod::ChunkRejection::NotEnoughDataPayload {
            declared: 3,
            available: 1
        }
    )
}

#[test]
fn parse_valid_chunk() {
    let input: Vec<u8> = create_byte_array(ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    let (parsed_chunk, rest) = ysl_mod::parse_to_chunk(&input).expect("valid operation");
    assert_eq!(
        parsed_chunk,
        ysl_mod::RawChunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }
    );
    assert_eq!(rest, &[]);
}

#[test]
fn parse_valid_chunk_extra_byte() {
    let mut input: Vec<u8> = create_byte_array(ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    input.extend_from_slice(&[0xA3]);
    let (parsed_chunk, rest) = ysl_mod::parse_to_chunk(&input).expect("valid operation");
    assert_eq!(
        parsed_chunk,
        ysl_mod::RawChunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }
    );
    assert_eq!(rest, &[0xA3]);
}

#[test]
fn parse_len_max() {
    let input: Vec<u8> = create_byte_array(ID_META, u32::MAX, &[0xA0, 0xA1, 0xA2]);
    let error_chunk = ysl_mod::parse_to_chunk(&input).unwrap_err();
    assert_eq!(
        error_chunk,
        ysl_mod::ChunkRejection::NotEnoughDataPayload {
            declared: u32::MAX,
            available: 3
        }
    )
}

#[test]
fn parse_consecutive_chunks() {
    let mut input: Vec<u8> = create_byte_array(ID_META, 3, &[0xA0, 0xA1, 0xA2]);
    input.extend_from_slice(&create_byte_array(ID_RHAI, 4, &[0xB0, 0xB1, 0xB2, 0xB3]));

    let (first_parsed_chunk, first_rest) =
        ysl_mod::parse_to_chunk(&input).expect("valid operation");
    assert_eq!(
        first_parsed_chunk,
        ysl_mod::RawChunk {
            four_cc: *b"META",
            payload: &[0xA0, 0xA1, 0xA2]
        }
    );
    let (second_parsed_chunk, second_rest) =
        ysl_mod::parse_to_chunk(first_rest).expect("valid operation");
    assert_eq!(
        second_parsed_chunk,
        ysl_mod::RawChunk {
            four_cc: *b"RHAI",
            payload: &[0xB0, 0xB1, 0xB2, 0xB3]
        }
    );
    assert_eq!(second_rest, &[]);
}
