pub const RESERVED_WINDOWS_DEVICE_NAME: [&str; 30] = [
    "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
    "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
];

pub fn is_valid_name_byte(b: u8) -> bool {
    matches!(b, b'-' | b'_' | b'a'..=b'z' | b'0'..=b'9')
}
