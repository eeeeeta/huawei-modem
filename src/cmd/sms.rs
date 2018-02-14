use {HuaweiModem};
use at::*;
use errors::*;
use futures::Future;
use pdu::{HexData, Pdu, DeliverPdu};
use util::HuaweiFromPrimitive;

#[repr(u8)]
#[derive(Fail, Debug, FromPrimitive, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageStatus {
    #[fail(display = "Unread")]
    ReceivedUnread = 0,
    #[fail(display = "Read")]
    ReceivedRead = 1,
    #[fail(display = "Unsent")]
    StoredUnsent = 2,
    #[fail(display = "Sent")]
    StoredSent = 3,
    #[fail(display = "All messages")]
    All = 4
}
#[derive(Clone, Debug)]
pub struct SmsMessage {
    pub status: MessageStatus,
    pub index: u32,
    pub pdu: DeliverPdu
}
pub fn list_sms_pdu(modem: &mut HuaweiModem, status: MessageStatus) -> impl Future<Item = Vec<SmsMessage>, Error = HuaweiError> {
    modem.send_raw(AtCommand::Equals {
        param: "+CMGL".into(),
        value: AtValue::Integer(status as u32)
    }).and_then(|pkt| {
        pkt.assert_ok()?;
        let mut cur = None;
        let mut ret = vec![];
        for resp in pkt.responses {
            match resp {
                AtResponse::InformationResponse { param, response } => {
                    assert_eq!(&param, "+CMGL");
                    let list = response.get_array()?;
                    let index = list.get(0)
                        .ok_or(HuaweiError::TypeMismatch)?
                        .get_integer()?;
                    let stat = list.get(1)
                        .ok_or(HuaweiError::TypeMismatch)?
                        .get_integer()?;
                    let stat = MessageStatus::from_integer(*stat)?;
                    cur = Some((*index, stat));
                },
                AtResponse::Unknown(ref st) => {
                    if st.trim().len() > 0 {
                        let cur = cur.take().ok_or(HuaweiError::TypeMismatch)?;
                        let hex = HexData::decode(st.trim())?;
                        let pdu = DeliverPdu::from_bytes(&hex)?;
                        ret.push(SmsMessage {
                            index: cur.0,
                            status: cur.1,
                            pdu
                        })
                    }
                },
                _ => {}
            }
        }
        Ok(ret)
    })
}
pub fn set_sms_textmode(modem: &mut HuaweiModem, text: bool) -> impl Future<Item = (), Error = HuaweiError> {
    modem.send_raw(AtCommand::Equals {
        param: "+CMGF".into(),
        value: AtValue::Integer(if text { 1 } else { 0 })
    }).and_then(|pkt| {
        pkt.assert_ok()?;
        Ok(())
    })
}
pub fn send_sms_textmode(modem: &mut HuaweiModem, to: String, msg: String) -> impl Future<Item = u32, Error = HuaweiError> {
    let text = format!("AT+CMGS=\"{}\"\n{}\x1A", to, msg);
    modem.send_raw(AtCommand::Text { text, expected: vec!["+CMGS".into()] })
        .and_then(|pkt| {
           let rpl = pkt.extract_named_response("+CMGS")?
               .get_integer()?;
           Ok(*rpl)
        })
}
pub fn send_sms_pdu(modem: &mut HuaweiModem, pdu: &Pdu) -> impl Future<Item = u32, Error = HuaweiError> {
    let (data, len) = pdu.as_bytes();
    let text = format!("AT+CMGS={}\n{}\x1A", len, HexData(&data));
    modem.send_raw(AtCommand::Text { text, expected: vec!["+CMGS".into()] })
        .and_then(|pkt| {
           let rpl = pkt.extract_named_response("+CMGS")?
               .get_integer()?;
           Ok(*rpl)
        })
}
