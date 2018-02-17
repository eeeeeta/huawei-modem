use std::fmt;
use std::str::FromStr;
use num::FromPrimitive;
use std::convert::{Infallible, TryFrom};
use errors::*;

macro_rules! check_offset {
    ($b:ident, $offset:ident, $reason:expr) => {
        if $b.get($offset).is_none() {
            return Err(HuaweiError::InvalidPdu(concat!("Offset check failed for: ", $reason)));
        }
    }
}

#[derive(Debug, Clone)]
pub struct UdhComponent {
    pub id: u8,
    pub data: Vec<u8>
}
#[derive(Debug, Clone)]
pub struct UserDataHeader {
    pub components: Vec<UdhComponent>
}
impl UserDataHeader {
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
impl TryFrom<u8> for AddressType {
    type Error = HuaweiError;
    fn try_from(b: u8) -> HuaweiResult<Self>  {
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
}
impl Into<u8> for AddressType {
    fn into(self) -> u8 {
        let mut ret: u8 = 0b1_000_0000;
        ret |= self.type_of_number as u8;
        ret |= self.numbering_plan_identification as u8;
        ret
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneNumber(pub Vec<u8>);
impl<'a> From<&'a [u8]> for PhoneNumber {
    fn from(b: &[u8]) -> Self {
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
}
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PduAddress {
    pub type_addr: AddressType,
    pub number: PhoneNumber
}
impl fmt::Display for PduAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // XXX: We don't display GSM numbers correctly.
        let prefix = match self.type_addr.type_of_number {
            TypeOfNumber::International => "+",
            TypeOfNumber::Gsm => "[GSM] ",
            _ => ""
        };
        write!(f, "{}", prefix)?;
        for b in self.number.0.iter() {
            write!(f, "{}", b)?;
        }
        Ok(())
    }
}
impl FromStr for PduAddress {
    type Err = Infallible;
    fn from_str(st: &str) -> Result<Self, Infallible> {
        let mut int = false;
        let buf = st.chars()
            .filter_map(|x| {
                match x {
                    '0'...'9' => Some(x as u8 - 48),
                    '+' => {
                        int = true;
                        None
                    },
                    _ => None
                }
            }).collect::<Vec<_>>();
        let ton = if int {
            TypeOfNumber::International
        }
        else {
            TypeOfNumber::Unknown
        };
        Ok(PduAddress {
            type_addr: AddressType {
                type_of_number: ton,
                numbering_plan_identification: NumberingPlanIdentification::IsdnTelephone
            },
            number: PhoneNumber(buf)
        })
    }
}
impl<'a> TryFrom<&'a [u8]> for PduAddress {
    type Error = HuaweiError;
    fn try_from(b: &[u8]) -> HuaweiResult<Self> {
        if b.len() < 2 {
            Err(HuaweiError::InvalidPdu("tried to make a PduAddress from less than 2 bytes"))?
        }
        let type_addr = AddressType::try_from(b[0])?;
        let number = PhoneNumber::from(&b[1..]);
        Ok(PduAddress { type_addr, number })
    }
}
impl PduAddress {
    pub fn as_bytes(&self, broken_len: bool) -> Vec<u8> {
        let mut ret = vec![];
        ret.push(self.type_addr.into());
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
impl From<u8> for PduFirstOctet {
    fn from(b: u8) -> Self {
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
}
impl Into<u8> for PduFirstOctet {
    fn into(self) -> u8 {
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
    pub fn encoding(&self) -> MessageEncoding {
        use self::DataCodingScheme::*;
        match *self {
            Standard { encoding, .. } => encoding,
            Reserved => MessageEncoding::Gsm7Bit,
            MessageWaitingDiscard { .. } => MessageEncoding::Gsm7Bit,
            MessageWaiting { ucs2, .. } => if ucs2 {
                MessageEncoding::Ucs2
            }
            else {
                MessageEncoding::Gsm7Bit
            }
        }
    }
}
impl From<u8> for DataCodingScheme {
    fn from(b: u8) -> Self {
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
}
impl Into<u8> for DataCodingScheme {
    fn into(self) -> u8 {
        use self::DataCodingScheme::*;
        match self {
            Standard { compressed, class, encoding } => {
                let mut ret = 0b0001_0000;
                if compressed {
                    ret |= 0b0010_0000;
                }
                ret |= class as u8;
                ret |= encoding as u8;
                ret
            },
            Reserved => 0b0100_0101,
            MessageWaiting { waiting, type_indication, ucs2 } => {
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
            MessageWaitingDiscard { waiting, type_indication } => {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliverPduFirstOctet {
    mti: MessageType,
    sri: bool,
    udhi: bool,
    rp: bool
}
impl From<u8> for DeliverPduFirstOctet {
    fn from(b: u8) -> Self {
        let mti = MessageType::from_u8(b & 0b000000_11)
            .expect("MessageType conversions should be exhaustive!");
        let sri = (b & 0b00100000) > 0;
        let udhi = (b & 0b01000000) > 0;
        let rp = (b & 0b01000000) > 0;
        DeliverPduFirstOctet { mti, sri, udhi, rp }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmscTimestamp {
    year: u8,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    timezone: u8
}
pub fn reverse_byte(b: u8) -> u8 {
    let units = b >> 4;
    let tens = b & 0b0000_1111;
    (tens * 10) + units
}
impl<'a> TryFrom<&'a [u8]> for SmscTimestamp {
    type Error = HuaweiError;
    fn try_from(b: &[u8]) -> HuaweiResult<Self> {
        if b.len() != 7 {
            Err(HuaweiError::InvalidPdu("SmscTimestamp must be 7 bytes long"))?
        }
        Ok(SmscTimestamp {
            year: reverse_byte(b[0]),
            month: reverse_byte(b[1]),
            day: reverse_byte(b[2]),
            hour: reverse_byte(b[3]),
            minute: reverse_byte(b[4]),
            second: reverse_byte(b[5]),
            timezone: reverse_byte(b[6]),
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliverPdu {
    pub sca: Option<PduAddress>,
    pub first_octet: DeliverPduFirstOctet,
    pub originating_address: PduAddress,
    pub dcs: DataCodingScheme,
    pub scts: SmscTimestamp,
    pub user_data: Vec<u8>,
    pub user_data_len: u8
}
impl DeliverPdu {
    pub fn get_message_data(&self) -> GsmMessageData {
        GsmMessageData {
            bytes: self.user_data.clone(),
            user_data_len: self.user_data_len,
            encoding: self.dcs.encoding(),
            udh: self.first_octet.udhi
        }
    }
}
impl<'a> TryFrom<&'a [u8]> for DeliverPdu {
    type Error = HuaweiError;
    fn try_from(b: &[u8]) -> HuaweiResult<Self> {
        let scalen = b[0];
        let mut offset: usize = scalen as usize + 1;
        let sca = if scalen > 0 {
            let o = offset - 1;
            check_offset!(b, o, "SCA");
            Some(PduAddress::try_from(&b[1..offset])?)
        }
        else {
            None
        };
        check_offset!(b, offset, "first octet");
        let first_octet = DeliverPduFirstOctet::from(b[offset]);
        offset += 1;
        check_offset!(b, offset, "originating address len");
        let destination_len = b[offset];
        offset += 1;
        let real_len = (destination_len / 2) + 1;
        let destination_end = offset + (real_len as usize);
        let de = destination_end - 1;
        check_offset!(b, de, "originating address");
        let originating_address = PduAddress::try_from(&b[offset..destination_end])?;
        if originating_address.type_addr.type_of_number == TypeOfNumber::Gsm {
            // XXX: What we really need to do is handle GSM numbers properly.
            // However, this lets us actually get text out of them, although
            // the sender is still mangled.
            offset += 1;
        }
        offset += real_len as usize;
        check_offset!(b, offset, "protocol identifier");
        let _pid = b[offset];
        offset += 1;
        check_offset!(b, offset, "data coding scheme");
        let dcs = DataCodingScheme::from(b[offset]);
        offset += 1;
        let scts_end = offset + 7;
        let ss = offset + 6;
        check_offset!(b, ss, "service center timestamp");
        let scts = SmscTimestamp::try_from(&b[offset..scts_end])?;
        offset += 7;
        check_offset!(b, offset, "user data len");
        let user_data_len = b[offset];
        offset += 1;
        let user_data = if b.get(offset).is_some() {
            b[offset..].to_owned()
        }
        else {
            vec![]
        };
        Ok(DeliverPdu {
            sca,
            first_octet,
            originating_address,
            dcs,
            scts,
            user_data,
            user_data_len
        })

    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
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
                udhi: msg.udh,
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
}
impl<'a> TryFrom<&'a [u8]> for Pdu {
    type Error = HuaweiError;
    fn try_from(b: &[u8]) -> HuaweiResult<Self> {
        let scalen = b[0];
        let mut offset: usize = scalen as usize + 1;
        let sca = if scalen > 0 {
            let o = offset - 1;
            check_offset!(b, o, "SCA");
            Some(PduAddress::try_from(&b[1..offset])?)
        }
        else {
            None
        };
        check_offset!(b, offset, "first octet");
        let first_octet = PduFirstOctet::from(b[offset]);
        offset += 1;
        check_offset!(b, offset, "message ID");
        let message_id = b[offset];
        offset += 1;
        check_offset!(b, offset, "destination len");
        let destination_len = b[offset];
        offset += 1;
        let real_len = (destination_len / 2) + destination_len % 2 + 1;
        let destination_end = offset + (real_len as usize);
        let de = destination_end - 1;
        check_offset!(b, de, "destination address");
        let destination = PduAddress::try_from(&b[offset..destination_end])?;
        offset += real_len as usize;
        check_offset!(b, offset, "protocol identifier");
        let _pid = b[offset];
        offset += 1;
        check_offset!(b, offset, "data coding scheme");
        let dcs = DataCodingScheme::from(b[offset]);
        offset += 1;
        let validity_period = if first_octet.vpf != VpFieldValidity::Invalid {
            check_offset!(b, offset, "validity period");
            let ret = b[offset];
            offset += 1;
            ret
        }
        else {
            0
        };
        check_offset!(b, offset, "user data len");
        let user_data_len = b[offset];
        offset += 1;
        let user_data = if b.get(offset).is_some() {
            b[offset..].to_owned()
        }
        else {
            vec![]
        };
        Ok(Pdu {
            sca,
            first_octet,
            message_id,
            destination,
            dcs,
            validity_period,
            user_data,
            user_data_len
        })
    }
}
impl Pdu {
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
        ret.push(self.first_octet.into());
        ret.push(self.message_id);
        ret.extend(self.destination.as_bytes(true));
        ret.push(0);
        ret.push(self.dcs.into());
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
impl<'a> HexData<'a> {
    pub fn decode(data: &str) -> HuaweiResult<Vec<u8>> {
        data.as_bytes()
            .chunks(2)
            .map(::std::str::from_utf8)
            .map(|x| {
                match x {
                    Ok(x) => u8::from_str_radix(x, 16)
                        .map_err(|_| HuaweiError::InvalidPdu("invalid hex string")),
                    Err(_) => Err(HuaweiError::InvalidPdu("invalid hex string"))
                }
            })
            .collect()
    }
}
pub fn split_buffers(buf: Vec<u8>, max_len: usize) -> Vec<Vec<u8>> {
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
#[derive(Debug, Clone)]
pub struct GsmMessageData {
    encoding: MessageEncoding,
    udh: bool,
    bytes: Vec<u8>,
    user_data_len: u8
}
#[derive(Debug, Clone)]
pub struct DecodedMessage {
    pub text: String,
    pub udh: Option<UserDataHeader>
}
impl GsmMessageData {
    pub fn encoding(&self) -> &MessageEncoding {
        &self.encoding
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn user_data_len(&self) -> u8 {
        self.user_data_len
    }
    pub fn decode_message(&self) -> HuaweiResult<DecodedMessage> {
        use encoding::{Encoding, DecoderTrap};
        use encoding::all::UTF_16BE;
        use gsm_encoding;
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
                let buf = decode_sms_7bit(&self.bytes[start..], padding);
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
    pub fn encode_message(msg: &str) -> Vec<GsmMessageData> {
        use encoding::{Encoding, EncoderTrap};
        use encoding::all::UTF_16BE;
        use rand;
        use gsm_encoding;

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
pub fn decode_sms_7bit(orig: &[u8], padding: usize) -> Vec<u8> {
    let mut ret = vec![0];
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
        let mut cur = ((data << (8 - chars_cur)) >> (8 - chars_cur)) << (7 - chars_cur);
        ret[i] |= cur;
        // XXX: This i == 152 condition is a hack. For some reason,
        // we need to push the last bit for full SMSes, but not for others?
        if j+1 < orig.len() || i == 152 {
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
pub fn encode_sms_7bit(orig: &[u8], padding: usize) -> Vec<u8> {
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

