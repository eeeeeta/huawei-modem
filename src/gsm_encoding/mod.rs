//! Utilities for dealing with the (annoying) GSM 7-bit encoding (GSM 03.38), and decoding/encoding message
//! data.
//!
//! "The annoying GSM 7-bit encoding" is otherwise known as [GSM
//! 03.38](https://en.wikipedia.org/wiki/GSM_03.38), and that Wikipedia article is pretty
//! informative.
//!
//! **NB:** SMS messages that are longer than the per-message character limit are sent & received
//! as [concatenated SMS](https://en.wikipedia.org/wiki/Concatenated_SMS) messages. The various
//! functions in this module will attempt to warn you about this.

use crate::pdu::MessageEncoding;
use std::convert::TryFrom;
use crate::errors::*;

mod lookup_tables;
pub mod udh;

use self::udh::{UserDataHeader, UdhComponent};
use self::lookup_tables::*;

/// Decode a GSM 7-bit-encoded buffer into a string.
///
/// **Warning:** You need to unpack the string first; this method operates on unpacked septets, not
/// packed septets. See the `pdu` module for more.
///
/// This method is lossy, and doesn't complain about crap that it can't decode.
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
/// Tries to encode a character into the given destination buffer, returning `true` if the
/// character was successfully encoded, and `false` if the character cannot be represented in the
/// GSM 7-bit encoding.
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
/// Tries to encode a string as GSM 7-bit, returning a buffer of **unpacked** septets iff all of
/// the data in `input` was representable in the 7-bit encoding.
///
/// **Warning:** The output of this function is unsuitable for transmission across the network;
/// you need to pack the septets first! See the `pdu` module for more.
pub fn try_gsm_encode_string(input: &str) -> Option<Vec<u8>> {
    let mut ret = vec![];
    for c in input.chars() {
        if !try_gsm_encode_char(c, &mut ret) {
            return None;
        }
    }
    Some(ret)
}
fn split_buffers(buf: Vec<u8>, max_len: usize) -> Vec<Vec<u8>> {
    let mut ret = vec![];
    let mut cbuf = buf;
    while max_len < cbuf.len() {
        let split = cbuf.split_off(max_len);
        let old = ::std::mem::replace(&mut cbuf, split);
        ret.push(old);
    }
    ret.push(cbuf);
    ret
}
/// The 'data' portion of an SMS message - i.e. the text, for a simple message.
#[derive(Debug, Clone)]
pub struct GsmMessageData {
    pub(crate) encoding: MessageEncoding,
    pub(crate) udh: bool,
    pub(crate) bytes: Vec<u8>,
    pub(crate) user_data_len: u8
}
/// A decoded text mesasge, with optional user data header.
#[derive(Debug, Clone, Default)]
pub struct DecodedMessage {
    /// Decoded text.
    pub text: String,
    /// User data header. You'll want this to check if the message is concatenated, i.e. is part of
    /// a multi-part series.
    pub udh: Option<UserDataHeader>
}
impl GsmMessageData {
    /// Get the message encoding.
    pub fn encoding(&self) -> &MessageEncoding {
        &self.encoding
    }
    /// Get the underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Get the user data length.
    pub fn user_data_len(&self) -> u8 {
        self.user_data_len
    }
    /// Attempt to decode this message.
    pub fn decode_message(&self) -> HuaweiResult<DecodedMessage> {
        use encoding::{Encoding, DecoderTrap};
        use encoding::all::UTF_16BE;
        use crate::gsm_encoding;
        let mut padding = 0;
        let mut start = 0;
        let mut udh = None;
        if self.udh {
            if self.bytes.len() < 1 {
                Err(HuaweiError::InvalidPdu("UDHI specified, but no data"))?
            }
            let udhl = self.bytes[0] as usize;
            padding = 7 - (((udhl + 1) * 8) % 7);
            start = udhl + 1;
            if self.bytes.len() < start {
                Err(HuaweiError::InvalidPdu("UDHL goes past end of data"))?
            }
            udh = Some(UserDataHeader::try_from(&self.bytes[1..start])?);
        }
        if self.bytes.get(start).is_none() {
            return Ok(DecodedMessage {
                text: String::new(),
                udh
            });
        }
        match self.encoding {
            MessageEncoding::Gsm7Bit => {
                let buf = decode_sms_7bit(&self.bytes[start..], padding, self.user_data_len as _);
                Ok(DecodedMessage {
                    text: gsm_encoding::gsm_decode_string(&buf),
                    udh
                })
            },
            MessageEncoding::Ucs2 => {
                Ok(DecodedMessage {
                    text: UTF_16BE.decode(&self.bytes[start..], DecoderTrap::Replace).unwrap(),
                    udh
                })
            },
            x => Err(HuaweiError::UnsupportedEncoding(x, self.bytes.clone()))
        }
    }
    /// Encode an arbitrary string of text into one, or multiple, GSM message data segments.
    ///
    /// If this function returns more than one bit of data, it means it's been split into multiple
    /// concatenated parts for you, and you'll need to send each part individually in order, as
    /// part of a new `Pdu` to your desired recipient.
    pub fn encode_message(msg: &str) -> Vec<GsmMessageData> {
        use encoding::{Encoding, EncoderTrap};
        use encoding::all::UTF_16BE;
        use rand;
        use crate::gsm_encoding;

        if let Some(buf) = gsm_encoding::try_gsm_encode_string(msg) {
            let user_data_len = buf.len();
            if user_data_len > 160 {
                // time to make a Concatenated SMS
                let bufs = split_buffers(buf, 153);
                let csms_ref = rand::random::<u8>();
                let num_parts = bufs.len() as u8;
                bufs.into_iter()
                    .enumerate()
                    .map(|(i, buf)| {
                        let udh = UserDataHeader {
                            components: vec![UdhComponent {
                                id: 0,
                                data: vec![csms_ref, num_parts, i as u8 + 1]
                            }]
                        };
                        let mut ret = udh.as_bytes();
                        let padding = 7 - ((ret.len() * 8) % 7);
                        let len = ((ret.len() * 8) + padding + (buf.len() * 7)) / 7;
                        let enc = encode_sms_7bit(&buf, padding);
                        ret.extend(enc);
                        GsmMessageData {
                            encoding: MessageEncoding::Gsm7Bit,
                            bytes: ret,
                            udh: true,
                            user_data_len: len as u8
                        }
                    })
                    .collect()
            }
            else {
                let buf = encode_sms_7bit(&buf, 0);
                vec![GsmMessageData {
                    encoding: MessageEncoding::Gsm7Bit,
                    bytes: buf,
                    udh: false,
                    user_data_len: user_data_len as u8
                }]
            }
        }
        else {
            let buf = UTF_16BE.encode(msg, EncoderTrap::Replace).unwrap();
            let user_data_len = buf.len();
            if user_data_len > 140 {
                // time to make a Concatenated SMS
                let bufs = split_buffers(buf, 134);
                let csms_ref = rand::random::<u8>();
                let num_parts = bufs.len() as u8;
                bufs.into_iter()
                    .enumerate()
                    .map(|(i, buf)| {
                        let udh = UserDataHeader {
                            components: vec![UdhComponent {
                                id: 0,
                                data: vec![csms_ref, num_parts, i as u8 + 1]
                            }]
                        };
                        let mut ret = udh.as_bytes();
                        ret.extend(buf);
                        let len = ret.len();
                        GsmMessageData {
                            encoding: MessageEncoding::Ucs2,
                            bytes: ret,
                            udh: true,
                            user_data_len: len as u8
                        }
                    })
                    .collect()
            }
            else {
                vec![GsmMessageData {
                    encoding: MessageEncoding::Ucs2,
                    bytes: buf,
                    udh: false,
                    user_data_len: user_data_len as u8
                }]
            }
        }
    }
}
// This function wins the "I spent a *goddamn hour* debugging this crap" award.
// The best part? The bug wasn't even in this function...!
pub(crate) fn decode_sms_7bit(orig: &[u8], padding: usize, len: usize) -> Vec<u8> {
    let mut ret = vec![0];
    // Number of bits in the current octet that come from the current septet.
    let mut chars_cur = 7;
    let mut i = 0;
    if padding > 0 && orig.len() > 0 {
        chars_cur = padding;
    }
    for (j, data) in orig.iter().enumerate() {
        if chars_cur == 0 {
            chars_cur = 7;
            ret.push(0);
            i += 1;
        }
        let next = data >> chars_cur;
        let cur = ((data << (8 - chars_cur)) >> (8 - chars_cur)) << (7 - chars_cur);
        ret[i] |= cur;
        if j+1 < orig.len() || ret.len() < len {
            ret.push(next);
        }
        chars_cur -= 1;
        i += 1;
    }
    if padding > 0 && ret.len() > 0 {
        ret.remove(0);
    }
    ret
}
pub(crate) fn encode_sms_7bit(orig: &[u8], padding: usize) -> Vec<u8> {
    let mut ret = vec![];
    // Number of bits in the current octet that come from the current septet.
    let mut chars_cur = 7;
    if padding > 0 && orig.len() > 0 {
        chars_cur = padding;
        let cur = orig[0] << padding;
        ret.push(cur);
        chars_cur -= 1;
    }
    for (i, data) in orig.iter().enumerate() {
        if chars_cur == 0 {
            chars_cur = 7;
            continue;
        }
        let mut cur = (*data & 0b01111111) >> (7 - chars_cur);
        let next = if let Some(n) = orig.get(i+1) {
            *n << chars_cur
        }
        else {
            0
        };
        cur |= next;
        ret.push(cur);
        chars_cur -= 1;
    }
    ret
}

