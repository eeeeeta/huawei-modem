use {HuaweiModem};
use at::*;
use errors::*;
use futures::Future;
use util::HuaweiFromPrimitive;

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
    modem.send_raw(AtCommand::Text(text))
        .and_then(|pkt| {
           let rpl = pkt.extract_named_response("+CMGS")?
               .get_integer()?;
           Ok(*rpl)
        })
}
