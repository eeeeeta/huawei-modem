//! Utilities for dealing with User Data Headers (used for concatenated SMS, among other things)
//! inside messages.
//!
//! [This Wikipedia article](https://en.wikipedia.org/wiki/User_Data_Header) explains what this is
//! for pretty well. Most uses of the UDH are vestigial; nowadays it's mostly useful for sending
//! concatenated SMS.
use std::convert::TryFrom;
use crate::errors::*;

/// Component of a User Data Header.
#[derive(Debug, Clone)]
pub struct UdhComponent {
    /// Component identifier.
    pub id: u8,
    /// Component data.
    pub data: Vec<u8>
}
/// A User Data Header itself.
///
/// You'll likely just want to call `get_concatenated_sms_data` on this to check whether the
/// message is concatenated.
#[derive(Debug, Clone)]
pub struct UserDataHeader {
    pub components: Vec<UdhComponent>
}
/// Data about a concatenated SMS.
#[derive(Debug, Clone)]
pub struct ConcatenatedSmsData {
    /// Reference that identifies which message this is a part of - this is like an ID for the
    /// whole message.
    pub reference: u16,
    /// How many parts to the message exist (e.g. 2).
    pub parts: u8,
    /// Which part this is (e.g. 1 of 2).
    pub sequence: u8
}
impl UserDataHeader {
    /// If there is concatenated SMS data in this header, return it.
    pub fn get_concatenated_sms_data(&self) -> Option<ConcatenatedSmsData> {
        for comp in self.components.iter() {
            if comp.id == 0 && comp.data.len() == 3 {
                return Some(ConcatenatedSmsData {
                    reference: comp.data[0] as _,
                    parts: comp.data[1],
                    sequence: comp.data[2]
                });
            }
            if comp.id == 8 && comp.data.len() == 4 {
                let reference = ((comp.data[0] as u16) << 8) | (comp.data[1] as u16);
                return Some(ConcatenatedSmsData {
                    reference,
                    parts: comp.data[2],
                    sequence: comp.data[3]
                });
            }
        }
        None
    }
    /// Serialize this UDH to wire format.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut ret = vec![];
        for comp in self.components.iter() {
            ret.push(comp.id);
            ret.push(comp.data.len() as u8);
            ret.extend(comp.data.clone());
        }
        let len = ret.len() as u8;
        ret.insert(0, len);
        ret
    }
}
impl<'a> TryFrom<&'a [u8]> for UserDataHeader {
    type Error = HuaweiError;
    /// Accepts a UDH *without* the UDH Length octet at the start.
    fn try_from(b: &[u8]) -> HuaweiResult<Self> {
        let mut offset = 0;
        let mut ret = vec![];
        loop {
            if b.get(offset).is_none() {
                break;
            }
            let id = b[offset];
            offset += 1;
            check_offset!(b, offset, "UDH component length");
            let len = b[offset];
            let end = offset + len as usize + 1;
            offset += 1;
            let o = end - 1;
            check_offset!(b, o, "UDH component data");
            let data = b[offset..end].to_owned();
            offset = end;
            ret.push(UdhComponent { id, data });
        }
        Ok(UserDataHeader {
            components: ret
        })
    }
}

