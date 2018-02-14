pub static GSM_EXTENDED_ENCODING_TABLE: [(char, u8); 9] = [
    ('^', 0x14),
    ('{', 0x28),
    ('}', 0x29),
    ('\\', 0x2F),
    ('[', 0x3C),
    ('~', 0x3D),
    (']', 0x3E),
    ('|', 0x40),
    ('\u{20AC}', 0x65)
];
pub static GSM_ENCODING_TABLE: [(char, u8); 65] = [
    ('@', 0x00),
    ('\u{00A3}', 0x01),
    ('$', 0x02),
    ('\u{00A5}', 0x03),
    ('è', 0x04),
    ('é', 0x05),
    ('ù', 0x06),
    ('ì', 0x07),
    ('ò', 0x08),
    ('\u{00C7}', 0x09),
    ('\n', 0x0a),
    ('\u{00D8}', 0x0b),
    ('\u{00F8}', 0x0c),
    ('\r', 0x0d),
    ('\u{00C5}', 0x0e),
    ('\u{00E5}', 0x0f),
    ('\u{0394}', 0x10),
    ('_', 0x11),
    ('\u{03A6}', 0x12),
    ('Γ', 0x13),
    ('Λ', 0x14),
    ('Ω', 0x15),
    ('Π', 0x16),
    ('Ψ', 0x17),
    ('Σ', 0x18),
    ('Θ', 0x19),
    ('Ξ', 0x1A),
    ('Æ', 0x1C),
    ('æ', 0x1D),
    ('ß', 0x1E),
    ('É', 0x1F),
    (' ', 0x20),
    ('!', 0x21),
    ('"', 0x22),
    ('#', 0x23),
    ('¤', 0x24),
    ('%', 0x25),
    ('&', 0x26),
    ('\'', 0x27),
    ('(', 0x28),
    (')', 0x29),
    ('*', 0x2A),
    ('+', 0x2B),
    (',', 0x2C),
    ('-', 0x2D),
    ('.', 0x2E),
    ('/', 0x2F),
    (':', 0x3A),
    (';', 0x3B),
    ('<', 0x3C),
    ('=', 0x3D),
    ('>', 0x3E),
    ('?', 0x3F),
    ('¡', 0x40),
    ('Ä', 0x5B),
    ('Ö', 0x5C),
    ('Ñ', 0x5D),
    ('Ü', 0x5E),
    ('§', 0x5F),
    ('¿', 0x60),
    ('ä', 0x7B),
    ('ö', 0x7C),
    ('ñ', 0x7D),
    ('ü', 0x7E),
    ('à', 0x7F)
];
pub fn gsm_decode_string(input: &[u8]) -> String {
    let mut ret = String::new();
    let mut skip = false;
    for (i, b) in input.iter().enumerate() {
        if skip {
            skip = false;
            continue;
        }
        match *b {
            b'A' ... b'Z' | b'a' ... b'z' | b'0' ... b'9' => {
                ret.push(*b as char);
            },
            0x1B => {
                if let Some(b) = input.get(i+1) {
                    for &(ch, val) in GSM_EXTENDED_ENCODING_TABLE.iter() {
                        if val == *b {
                            ret.push(ch);
                            skip = true;
                        }
                    }
                }
            },
            b => {
                for &(ch, val) in GSM_ENCODING_TABLE.iter() {
                    if val == b {
                        ret.push(ch);
                    }
                }
            }
        }
    }
    ret
}
pub fn try_gsm_encode_char(b: char, dest: &mut Vec<u8>) -> bool {
    match b {
        'A' ... 'Z' | 'a' ... 'z' | '0' ... '9' => {
            dest.push(b as u8);
            return true;
        },
        b => {
            for &(ch, val) in GSM_ENCODING_TABLE.iter() {
                if b == ch {
                    dest.push(val);
                    return true;
                }
            }
            for &(ch, val) in GSM_EXTENDED_ENCODING_TABLE.iter() {
                if b == ch {
                    dest.push(0x1B);
                    dest.push(val);
                    return true;
                }
            }
        }
    }
    false
}
pub fn try_gsm_encode_string(input: &str) -> Option<Vec<u8>> {
    let mut ret = vec![];
    for c in input.chars() {
        if !try_gsm_encode_char(c, &mut ret) {
            return None;
        }
    }
    Some(ret)
}
