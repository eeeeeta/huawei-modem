use std::fmt;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
    pub fn as_u8(self) -> u8 {
        let mut ret: u8 = 0b1_000_0000;
        ret |= self.type_of_number as u8;
        ret |= self.numbering_plan_identification as u8;
        ret
    }
}
#[derive(Debug, Clone)]
pub struct PhoneNumber(Vec<u8>);
impl PhoneNumber {
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
    type_addr: AddressType,
    number: PhoneNumber
}
impl PduAddress {
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageType {
    SmsDeliver = 0b000000_00,
    SmsCommand = 0b000000_10,
    SmsSubmit = 0b000000_01,
    Reserved = 0b000000_11
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageClass {
    Silent = 0b000000_00,
    StoreToNv = 0b000000_01,
    StoreToSim = 0b000000_10,
    StoreToTe = 0b000000_11
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageEncoding {
    Gsm7Bit = 0b0000_00_00,
    EightBit = 0b0000_01_00,
    Ucs2 = 0b0000_10_00,
    Reserved = 0b0000_11_00,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SimplisticDataCodingScheme {
    pub class: MessageClass,
    pub encoding: MessageEncoding
}
impl SimplisticDataCodingScheme {
    pub fn as_u8(self) -> u8 {
        let mut ret = 0b0001_0000;
        ret |= self.class as u8;
        ret |= self.encoding as u8;
        ret
    }
}
#[derive(Debug, Clone)]
pub struct Pdu {
    pub sca: PduAddress,
    pub first_octet: PduFirstOctet,
    pub message_id: u8,
    pub destination: PduAddress,
    pub dcs: SimplisticDataCodingScheme,
    pub validity_period: u8,
    pub user_data: Vec<u8>
}
impl Pdu {
    pub fn make_simple_message(smsc: &str, recipient: &str, msg: &str) -> Self {
        let smsc: Vec<u8> = smsc.chars().filter_map(|x| {
            match x {
                '0'...'9' => Some(x as u8 - 48),
                _ => None
            }
        }).collect();
        let recipient: Vec<u8> = recipient.chars().filter_map(|x| {
            match x {
                '0'...'9' => Some(x as u8 - 48),
                _ => None
            }
        }).collect();
        let msg = encode_sms_7bit(msg.as_bytes());
        Pdu {
            sca: PduAddress {
                type_addr: Default::default(),
                number: PhoneNumber(smsc)
            },
            first_octet: PduFirstOctet {
                mti: MessageType::SmsSubmit,
                rd: false,
                vpf: VpFieldValidity::Invalid,
                rp: false,
                udhi: false,
                srr: false
            },
            message_id: 0,
            destination: PduAddress {
                type_addr: Default::default(),
                number: PhoneNumber(recipient)
            },
            dcs: SimplisticDataCodingScheme {
                class: MessageClass::StoreToNv,
                encoding: MessageEncoding::Gsm7Bit
            },
            validity_period: 0,
            user_data: msg
        }
    }
    pub fn as_bytes(&self) -> (Vec<u8>, usize) {
        let mut ret = vec![];
        let sca = self.sca.as_bytes(false);
        let scalen = sca.len();
        ret.extend(sca);
        ret.push(self.first_octet.as_u8());
        ret.push(self.message_id);
        ret.extend(self.destination.as_bytes(true));
        ret.push(0);
        ret.push(self.dcs.as_u8());
        if self.first_octet.vpf != VpFieldValidity::Invalid {
            ret.push(self.validity_period);
        }
        ret.push(self.user_data.len() as u8);
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
pub fn encode_sms_7bit(orig: &[u8]) -> Vec<u8> {
    let mut ret = vec![];
    let mut chars_cur = 7;
    let mut chars_next = 1;
    for (i, data) in orig.iter().enumerate() {
        if chars_cur == 0 {
            chars_cur = 7;
            chars_next = 1;
            continue;
        }
        let mut cur = *data >> (7 - chars_cur);
        let next = if let Some(n) = orig.get(i+1) {
            *n << (8 - chars_next)
        }
        else {
            0
        };
        cur |= next;
        ret.push(cur);
        chars_cur -= 1;
        chars_next += 1;
    }
    ret.push(0);
    ret
}

