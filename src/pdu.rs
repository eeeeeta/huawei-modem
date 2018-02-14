use std::fmt;
use num::FromPrimitive;
use errors::*;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
pub enum TypeOfNumber {
    Unknown = 0b0_000_0000,
    International = 0b0_001_0000,
    National = 0b0_010_0000,
    Special = 0b0_011_0000,
    Gsm = 0b0_101_0000,
    Short = 0b0_110_0000,
    Reserved = 0b0_111_0000
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
pub enum NumberingPlanIdentification {
    NetworkDetermined = 0b0_000_0000,
    IsdnTelephone = 0b0_000_0001,
    Data = 0b0_000_0011,
    Telex = 0b0_000_0100,
    National = 0b0_000_1000,
    Private = 0b0_000_1001,
    Ermes = 0b0_000_1010
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AddressType {
    pub type_of_number: TypeOfNumber,
    pub numbering_plan_identification: NumberingPlanIdentification
}
impl Default for AddressType {
    fn default() -> Self {
        AddressType {
            type_of_number: TypeOfNumber::International,
            numbering_plan_identification: NumberingPlanIdentification::IsdnTelephone
        }
    }
}
impl AddressType {
    pub fn from_u8(b: u8) -> HuaweiResult<Self>  {
        let ton = b & 0b0_111_0000;
        let ton = TypeOfNumber::from_u8(ton)
            .ok_or(HuaweiError::InvalidPdu("invalid type_of_number"))?;
        let npi = b & 0b0_000_1111;
        let npi = NumberingPlanIdentification::from_u8(npi)
            .ok_or(HuaweiError::InvalidPdu("invalid numbering_plan_identification"))?;
        Ok(Self {
            type_of_number: ton,
            numbering_plan_identification: npi
        })
    }
    pub fn as_u8(self) -> u8 {
        let mut ret: u8 = 0b1_000_0000;
        ret |= self.type_of_number as u8;
        ret |= self.numbering_plan_identification as u8;
        ret
    }
}
#[derive(Debug, Clone)]
pub struct PhoneNumber(pub Vec<u8>);
impl PhoneNumber {
    pub fn from_bytes(b: &[u8]) -> Self {
        let mut ret = vec![];
        for b in b.iter() {
            let first = b & 0b0000_1111;
            let second = (b & 0b1111_0000) >> 4;
            ret.push(first);
            if second != 0b0000_1111 {
                ret.push(second);
            }
        }
        PhoneNumber(ret)
    }
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut ret = vec![];
        let mut cur = 0b0000_0000;
        for (i, b) in self.0.iter().enumerate() {
            let mut b = *b;
            if i % 2 == 0 {
                cur |= b;
            }
            else {
                b = b << 4;
                cur |= b;
                ret.push(cur);
                cur = 0b0000_0000;
            }
        }
        if self.0.len() % 2 != 0 {
            cur |= 0b1111_0000;
            ret.push(cur);
        }
        ret
    }
}
#[derive(Debug, Clone)]
pub struct PduAddress {
    pub type_addr: AddressType,
    pub number: PhoneNumber
}
impl PduAddress {
    pub fn from_str(st: &str) -> Self {
        let buf = st.chars()
            .filter_map(|x| {
                match x {
                    '0'...'9' => Some(x as u8 - 48),
                    _ => None
                }
            }).collect::<Vec<_>>();
        PduAddress {
            type_addr: Default::default(),
            number: PhoneNumber(buf)
        }
    }
    pub fn from_bytes(b: &[u8]) -> HuaweiResult<Self> {
        if b.len() < 2 {
            Err(HuaweiError::InvalidPdu("tried to make a PduAddress from less than 2 bytes"))?
        }
        let type_addr = AddressType::from_u8(b[0])?;
        let number = PhoneNumber::from_bytes(&b[1..]);
        Ok(PduAddress { type_addr, number })
    }
    pub fn as_bytes(&self, broken_len: bool) -> Vec<u8> {
        let mut ret = vec![];
        ret.push(self.type_addr.as_u8());
        ret.extend(self.number.as_bytes());
        let len = if broken_len {
            self.number.0.len()
        } else {
            ret.len()
        };
        ret.insert(0, len as u8);
        ret
    }
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
pub enum MessageType {
    SmsDeliver = 0b000000_00,
    SmsCommand = 0b000000_10,
    SmsSubmit = 0b000000_01,
    Reserved = 0b000000_11
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
pub enum VpFieldValidity {
    Invalid = 0b0000_00_00,
    Relative = 0b0000_10_00,
    Enhanced = 0b0000_01_00,
    Absolute = 0b0000_11_00,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PduFirstOctet {
    mti: MessageType,
    rd: bool,
    vpf: VpFieldValidity,
    srr: bool,
    udhi: bool,
    rp: bool
}
impl PduFirstOctet {
    pub fn from_u8(b: u8) -> Self {
        let rd = (b & 0b00000100) > 0;
        let srr = (b & 0b00100000) > 0;
        let udhi = (b & 0b01000000) > 0;
        let rp = (b & 0b10000000) > 0;
        let mti = MessageType::from_u8(b & 0b000000_11)
            .expect("MessageType conversions should be exhaustive!");
        let vpf = VpFieldValidity::from_u8(b & 0b0000_11_00)
            .expect("VpFieldValidity conversions should be exhaustive!");
        PduFirstOctet { rd, srr, udhi, rp, mti, vpf }
    }
    pub fn as_u8(self) -> u8 {
        let mut ret = 0b0000_0000;
        ret |= self.mti as u8;
        ret |= self.vpf as u8;
        if self.rd {
            ret |= 0b00000100;
        }
        if self.srr {
            ret |= 0b00100000;
        }
        if self.udhi {
            ret |= 0b01000000;
        }
        if self.rp {
            ret |= 0b10000000;
        }
        ret
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DataCodingScheme {
    Standard {
        compressed: bool,
        class: MessageClass,
        encoding: MessageEncoding
    },
    Reserved,
    MessageWaitingDiscard {
        waiting: bool,
        type_indication: MessageWaitingType,
    },
    MessageWaiting {
        waiting: bool,
        type_indication: MessageWaitingType,
        ucs2: bool
    }
}
impl DataCodingScheme {
    pub fn from_u8(b: u8) -> Self {
        if (b & 0b1100_0000) == 0b0000_0000 {
            let compressed = (b & 0b0010_0000) > 0;
            let reserved = (b & 0b0001_0000) > 0;
            let class = if reserved {
                // XXX: No default is actually specified in the Huawei spec; I just chose
                // StoreToNv, because that's what we send by default.
                MessageClass::StoreToNv
            }
            else {
                MessageClass::from_u8(b & 0b0000_0011)
                    .expect("MessageClass conversions should be exhaustive!")
            };
            let encoding = MessageEncoding::from_u8(b & 0b0000_1100)
                .expect("MessageEncoding conversions should be exhaustive!");
            DataCodingScheme::Standard { compressed, class, encoding }
        }
        else if (b & 0b1111_0000) == 0b1111_0000 {
            let compressed = false;
            let class = MessageClass::from_u8(b & 0b0000_0011)
                .expect("MessageClass conversions should be exhaustive!");
            let encoding = if (b & 0b0000_0100) > 0 {
                MessageEncoding::Gsm7Bit
            }
            else {
                MessageEncoding::EightBit
            };
            DataCodingScheme::Standard { compressed, class, encoding }
        }
        else if (b & 0b1111_0000) == 0b1100_0000 {
            let waiting = (b & 0b0000_1000) > 0;
            let type_indication = MessageWaitingType::from_u8(b & 0b0000_0011)
                .expect("MessageWaitingType conversions should be exhaustive!");
            DataCodingScheme::MessageWaitingDiscard { waiting, type_indication }
        }
        else if (b & 0b1111_0000) == 0b1101_0000 || (b & 0b1111_0000) == 0b1110_0000 {
            let ucs2 = (b & 0b1111_0000) == 0b1110_0000;
            let waiting = (b & 0b0000_1000) > 0;
            let type_indication = MessageWaitingType::from_u8(b & 0b0000_0011)
                .expect("MessageWaitingType conversions should be exhaustive!");
            DataCodingScheme::MessageWaiting { ucs2, waiting, type_indication }
        }
        else {
            DataCodingScheme::Reserved
        }
    }
    pub fn as_u8(&self) -> u8 {
        match *self {
            DataCodingScheme::Standard { compressed, class, encoding } => {
                let mut ret = 0b0001_0000;
                if compressed {
                    ret |= 0b0010_0000;
                }
                ret |= class as u8;
                ret |= encoding as u8;
                ret
            },
            DataCodingScheme::Reserved => 0b0100_0101,
            DataCodingScheme::MessageWaiting { waiting, type_indication, ucs2 } => {
                let mut ret = if ucs2 {
                    0b1110_0000
                }
                else {
                    0b1101_0000
                };
                if waiting {
                    ret |= 0b0000_1000;
                }
                ret |= type_indication as u8;
                ret
            },
            DataCodingScheme::MessageWaitingDiscard { waiting, type_indication } => {
                let mut ret = 0b1100_0000;
                if waiting {
                    ret |= 0b0000_1000;
                }
                ret |= type_indication as u8;
                ret
            }
        }
    }
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
pub enum MessageWaitingType {
    Voice = 0b000000_00,
    Fax = 0b000000_01,
    Email = 0b000000_10,
    Unknown = 0b000000_11
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
pub enum MessageClass {
    Silent = 0b000000_00,
    StoreToNv = 0b000000_01,
    StoreToSim = 0b000000_10,
    StoreToTe = 0b000000_11
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
pub enum MessageEncoding {
    Gsm7Bit = 0b0000_00_00,
    EightBit = 0b0000_01_00,
    Ucs2 = 0b0000_10_00,
    Reserved = 0b0000_11_00,
}
#[derive(Debug, Clone)]
pub struct Pdu {
    pub sca: Option<PduAddress>,
    pub first_octet: PduFirstOctet,
    pub message_id: u8,
    pub destination: PduAddress,
    pub dcs: DataCodingScheme,
    pub validity_period: u8,
    pub user_data: Vec<u8>,
    pub user_data_len: u8
}
impl Pdu {
    pub fn set_sca(&mut self, sca: PduAddress) {
        self.sca = Some(sca);
    }
    pub fn make_simple_message(recipient: PduAddress, msg: GsmMessageData) -> Self {
        Pdu {
            sca: None,
            first_octet: PduFirstOctet {
                mti: MessageType::SmsSubmit,
                rd: false,
                vpf: VpFieldValidity::Invalid,
                rp: false,
                udhi: false,
                srr: false
            },
            message_id: 0,
            destination: recipient,
            dcs: DataCodingScheme::Standard {
                compressed: false,
                class: MessageClass::StoreToNv,
                encoding: msg.encoding
            },
            validity_period: 0,
            user_data: msg.bytes,
            user_data_len: msg.user_data_len as u8
        }
    }
    pub fn as_bytes(&self) -> (Vec<u8>, usize) {
        let mut ret = vec![];
        let mut scalen = 1;
        if let Some(ref sca) = self.sca {
            let sca = sca.as_bytes(false);
            scalen = sca.len();
            ret.extend(sca);
        }
        else {
            ret.push(0);
        }
        ret.push(self.first_octet.as_u8());
        ret.push(self.message_id);
        ret.extend(self.destination.as_bytes(true));
        ret.push(0);
        ret.push(self.dcs.as_u8());
        if self.first_octet.vpf != VpFieldValidity::Invalid {
            ret.push(self.validity_period);
        }
        ret.push(self.user_data_len);
        ret.extend(self.user_data.clone());
        let tpdu_len = ret.len() - scalen;
        (ret, tpdu_len)
    }
}
pub struct HexData<'a>(pub &'a [u8]);
impl<'a> fmt::Display for HexData<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       for b in self.0.iter() {
           write!(f, "{:02X}", b)?;
       }
       Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct GsmMessageData {
    encoding: MessageEncoding,
    bytes: Vec<u8>,
    user_data_len: usize
}
impl GsmMessageData {
    pub fn encoding(&self) -> &MessageEncoding {
        &self.encoding
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn user_data_len(&self) -> usize {
        self.user_data_len
    }
    pub fn encode_message(msg: &str) -> GsmMessageData {
        use encoding::{Encoding, EncoderTrap};
        use encoding::all::UTF_16BE;
        use gsm_encoding;

        if let Some(buf) = gsm_encoding::try_gsm_encode_string(msg) {
            let user_data_len = buf.len();
            let buf = encode_sms_7bit(&buf);
            GsmMessageData {
                encoding: MessageEncoding::Gsm7Bit,
                bytes: buf,
                user_data_len
            }
        }
        else {
            let buf = UTF_16BE.encode(msg, EncoderTrap::Replace).unwrap();
            let user_data_len = buf.len();
            GsmMessageData {
                encoding: MessageEncoding::Ucs2,
                bytes: buf,
                user_data_len
            }
        }
    }
}
pub fn encode_sms_7bit(orig: &[u8]) -> Vec<u8> {
    let mut ret = vec![];
    // Number of bits in the current octet that come from the current septet.
    let mut chars_cur = 7;
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

