use {HuaweiModem};
use at::*;
use errors::*;
use futures::Future;
use pdu::{HexData, Pdu, AddressType, DeliverPdu};
use convert::TryFrom;
use util::HuaweiFromPrimitive;

/// The storage status of an SMS message.
#[repr(u8)]
#[derive(Fail, Debug, FromPrimitive, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageStatus {
    /// Received and unread.
    #[fail(display = "Unread")]
    ReceivedUnread = 0,
    /// Received and read.
    #[fail(display = "Read")]
    ReceivedRead = 1,
    /// Outgoing and unsent.
    #[fail(display = "Unsent")]
    StoredUnsent = 2,
    /// Outgoing and sent.
    #[fail(display = "Sent")]
    StoredSent = 3,
    /// Any kind (used for `list_sms_pdu` only).
    #[fail(display = "All messages")]
    All = 4
}
/// Controls whether to notify the TE about new messages.
#[repr(u8)]
#[derive(Debug, FromPrimitive, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NewMessageNotification {
    /// Buffer new message indications in the ME, overwriting old indications if necessary.
    BufferInMe = 0,
    /// Send SMS-DELIVER indications to the TE, discarding them if they cannot be sent
    /// (for example, when in online data mode)
    SendDirectlyOrDiscard = 1,
    /// Send SMS-DELIVER indications to the TE, buffering them and sending them later if they
    /// cannot be sent.
    SendDirectlyOrBuffer = 2
}
/// Controls how new messages are saved, and how indications are sent to the TE.
#[repr(u8)]
#[derive(Debug, FromPrimitive, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NewMessageStorage {
    /// Do not route any SMS-DELIVER indications to the TE.
    RouteNothing = 0,
    /// Store SMS-DELIVER indications on the MT, and send a `+CMTI: <mem>,<index>` URC to
    /// the TE.
    StoreAndNotify = 1,
    /// Directly forward the SMS-DELIVER indication to the TE, sending a `+CMT:
    /// [<reserved>],<length><CR><LF><pdu>` URC to the TE.
    SendDirectly = 2,
    /// Store SMS-DELIVER indications on the MT, but don't notify the TE.
    StoreAndDiscardNotification = 3
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeletionOptions {
    /// Delete the message stored at the index specified.
    Indexed(u32),
    /// Delete all read messages, keeping unread, sent, and unsent ones.
    DeleteRead,
    /// Delete all read and sent messages, keeping unread and unsent ones.
    DeleteReadAndSent,
    /// Delete all read, sent, and unsent messages, keeping unread ones.
    DeleteReadAndOutgoing,
    /// Delete all messages.
    DeleteAll
}
#[derive(Clone, Debug)]
pub struct SmsMessage {
    pub status: MessageStatus,
    pub index: u32,
    pub pdu: DeliverPdu
}
pub fn set_new_message_indications(modem: &mut HuaweiModem, mode: NewMessageNotification, mt: NewMessageStorage) -> impl Future<Item = (), Error = HuaweiError> {
    modem.send_raw(AtCommand::Equals {
        param: "+CNMI".into(),
        value: AtValue::Array(vec![
            AtValue::Integer(mode as u32),
            AtValue::Integer(mt as u32)
        ])
    }).and_then(|pkt| {
        pkt.assert_ok()?;
        Ok(())
    })
}
pub fn set_smsc_addr(modem: &mut HuaweiModem, sca: String, tosca: Option<AddressType>) -> impl Future<Item = (), Error = HuaweiError> {
    let mut arr = vec![AtValue::String(sca)];
    if let Some(t) = tosca {
        let t: u8 = t.into();
        arr.push(AtValue::Integer(t as u32));
    }
    modem.send_raw(AtCommand::Equals {
        param: "+CSCA".into(),
        value: AtValue::Array(arr)
    }).and_then(|pkt| {
        pkt.assert_ok()?;
        Ok(())
    })
}
pub fn del_sms_pdu(modem: &mut HuaweiModem, del: DeletionOptions) -> impl Future<Item = (), Error = HuaweiError> {
    use self::DeletionOptions::*;

    let (index, delflag) = match del {
        Indexed(i) => (i, 0),
        DeleteRead => (0, 1),
        DeleteReadAndSent => (0, 2),
        DeleteReadAndOutgoing => (0, 3),
        DeleteAll => (0, 4)
    };
    modem.send_raw(AtCommand::Equals {
        param: "+CMGD".into(),
        value: AtValue::Array(vec![AtValue::Integer(index), AtValue::Integer(delflag)])
    }).and_then(|pkt| {
        pkt.assert_ok()?;
        Ok(())
    })
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
                        let pdu = DeliverPdu::try_from(&hex as &[u8])?;
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
