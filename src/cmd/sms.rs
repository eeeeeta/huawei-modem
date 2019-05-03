//! Receiving and managing SMS messages.
//!
//! Modems typically support *PDU mode* SMS commands - which operate using SMS PDUs (Protocol Data
//! Units) - and *text mode* ones. PDUs are what you want, if your library can handle them (this one can; see the `pdu`
//! module), since they lets you handle things like concatenated messages and emoji.
//!
//! Due to issues with properly parsing text-mode SMS responses without breaking, this library does
//! *not* let you list messages in text mode, making it rather a pain to use.
//!
//! However, text mode *is* ostensibly useful if all you want to do is send simple 7-bit ASCII
//! messages, and you don't care about receiving.
//!
//! **NB:** You *MUST* configure the modem for PDU mode by calling `send_sms_textmode` with `false`
//! as argument before sending PDU-mode commands. Failure to do so will result in some fun times.
use crate::{HuaweiModem};
use crate::at::*;
use crate::errors::*;
use futures::Future;
use crate::pdu::{HexData, Pdu, AddressType, DeliverPdu};
use std::convert::TryFrom;
use crate::util::HuaweiFromPrimitive;

/// The storage status of an SMS message (returned in `AT+CMGL`).
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
/// Controls whether to notify the TE about new messages (from `AT+CNMI`).
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
    ///
    /// If you're aiming to use new message notification, this is probably what you want.
    SendDirectlyOrBuffer = 2
}
/// Controls how new messages are saved, and how indications are sent to the TE (from `AT+CNMI`).
#[repr(u8)]
#[derive(Debug, FromPrimitive, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NewMessageStorage {
    /// Do not route any SMS-DELIVER indications to the TE.
    RouteNothing = 0,
    /// Store SMS-DELIVER indications on the MT, and send a `+CMTI: <mem>,<index>` URC to
    /// the TE.
    ///
    /// If you're aiming to use new message notification, this is probably what you want (and is
    /// the only thing that has really been tested apart from `RouteNothing`).
    StoreAndNotify = 1,
    /// Directly forward the SMS-DELIVER indication to the TE, sending a `+CMT:
    /// [<reserved>],<length><CR><LF><pdu>` URC to the TE.
    SendDirectly = 2,
    /// Store SMS-DELIVER indications on the MT, but don't notify the TE.
    StoreAndDiscardNotification = 3
}
/// Controls which messages to delete (in `AT+CMGD`).
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
/// An SMS message, received from a listing.
#[derive(Clone, Debug)]
pub struct SmsMessage {
    /// The message status (read, unread, etc.)
    pub status: MessageStatus,
    /// The message's index in the modem's memory (useful if you want to delete it later).
    pub index: u32,
    /// The message's raw SMS PDU.
    ///
    /// Most implementations won't want to care about this, unless you want to store it somewhere
    /// verbatim for whatever reason and parse it again later.
    pub raw_pdu: Vec<u8>,
    /// The decoded SMS PDU (Protocol Data Unit).
    ///
    /// This contains everything you probably want to get out of the message, like the sender and
    /// the message text. Have a look at the documentation on `DeliverPdu` to figure out how to get
    /// stuff out of it!
    pub pdu: DeliverPdu
}
/// Controls whether to send new message indications to the TE when new messages arrive. Useful to
/// avoid polling. (`AT+CNMI`)
///
/// Basically, playing with this setting means you can get the modem to send you a URC (c.f.
/// `HuaweiModem::take_urc_rx`) when you get new messages, allowing you to intelligently list
/// messages only when they're delivered instead of polling all the damn time and suffering delays.
///
/// Looking at the docs on the two `enums` provided as arguments will give you an idea as to which
/// settings you'd want to use for this. 
/// 
/// Also note that this **may not necessarily be supported** by all modems! (in which case you'll have
/// to fall back on polling).
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
/// Set the address of the SMS Service Center (`AT+CSCA`).
///
/// You may need to configure this with the value provided by your network provider before being
/// able to send SMSes.
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
/// Delete a message from the modem's message store (`AT+CMGD`).
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
/// List SMSes from the modem's message store, in PDU mode (`AT+CMGL`).
///
/// The modem must be configured properly for PDU mode first. See the module-level documentation for
/// more information.
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
                            raw_pdu: hex.into(),
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
/// Set whether the modem will use text mode or not (`AT+CMGF`).
pub fn set_sms_textmode(modem: &mut HuaweiModem, text: bool) -> impl Future<Item = (), Error = HuaweiError> {
    modem.send_raw(AtCommand::Equals {
        param: "+CMGF".into(),
        value: AtValue::Integer(if text { 1 } else { 0 })
    }).and_then(|pkt| {
        pkt.assert_ok()?;
        Ok(())
    })
}
/// Send a message to a phone number, in text mode (`AT+CMGS`).
///
/// Using text mode is recommended against for all but the most simple of cases; see the module-level
/// documentation for more.
pub fn send_sms_textmode(modem: &mut HuaweiModem, to: String, msg: String) -> impl Future<Item = u32, Error = HuaweiError> {
    let text = format!("AT+CMGS=\"{}\"\n{}\x1A", to, msg);
    modem.send_raw(AtCommand::Text { text, expected: vec!["+CMGS".into()] })
        .and_then(|pkt| {
           let rpl = pkt.extract_named_response("+CMGS")?
               .get_integer()?;
           Ok(*rpl)
        })
}
/// Send a message to a phone number, in PDU mode (`AT+CMGS`).
///
/// See the `Pdu` documentation for information on how PDUs are made.
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
