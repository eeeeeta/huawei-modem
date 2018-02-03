use {HuaweiModem};
use at::*;
use errors::*;
use futures::Future;
use util::HuaweiFromPrimitive;

#[repr(u8)]
#[derive(Fail, Debug, FromPrimitive, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegistrationState {
    #[fail(display = "Not registered; not searching for a new operator")]
    NotRegisteredAndDisabled = 0,
    #[fail(display = "Registered; on home network")]
    RegisteredHome = 1,
    #[fail(display = "Not registered; searching for a new operator")]
    NotRegisteredSearching = 2,
    #[fail(display = "Registration denied")]
    RegistrationDenied = 3,
    #[fail(display = "Unknown registration state")]
    Unknown = 4,
    #[fail(display = "Registered; roaming")]
    RegisteredRoaming = 5
}
impl RegistrationState {
    pub fn is_registered(&self) -> bool {
        use self::RegistrationState::*;

        match *self {
            RegisteredHome => true,
            RegisteredRoaming => true,
            _ => false
        }
    }
}
#[repr(u8)]
#[derive(Fail, Debug, FromPrimitive, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModemOperationMode {
    #[fail(display = "Minimum functionality; disable RF but keep SIM power")]
    MinimumFunctionality = 0,
    #[fail(display = "Online mode")]
    OnlineMode = 1,
    #[fail(display = "Offline mode")]
    OfflineMode = 4,
    #[fail(display = "FTM mode")]
    FtmMode = 5,
    #[fail(display = "Restart modem")]
    Restart = 6,
    #[fail(display = "Disable RF")]
    DisableRf = 7
}
#[repr(u8)]
#[derive(Fail, Debug, Copy, Clone, PartialEq, Eq)]
pub enum PinState {
    #[fail(display = "Ready; no passwords required")]
    Ready,
    #[fail(display = "SIM PIN required")]
    SimPin,
    #[fail(display = "SIM PUK required")]
    SimPuk,
    #[fail(display = "SIM PIN2 required")]
    SimPin2,
    #[fail(display = "SIM PUK2 required")]
    SimPuk2
}
impl PinState {
    pub fn from_string(st: &str) -> HuaweiResult<Self> {
        let r = match st {
            "READY" => PinState::Ready,
            "SIM PIN" => PinState::SimPin,
            "SIM PUK" => PinState::SimPuk,
            "SIM PIN2" => PinState::SimPin2,
            "SIM PUK2" => PinState::SimPuk2,
            oth => return Err(HuaweiError::ValueOutOfRange(
                    AtValue::Unknown(oth.into())
            ))
        };
        Ok(r)
    }
}
pub fn get_registration(modem: &mut HuaweiModem) -> impl Future<Item = RegistrationState, Error = HuaweiError> {
    modem.send_raw(AtCommand::Read { param: "+CREG".into()})
        .and_then(|pkt| {
            let reg = pkt.extract_named_response("+CREG")?
                .get_array()?
                .get(1)
                .ok_or(HuaweiError::TypeMismatch)?
                .get_integer()?;
            let regst = RegistrationState::from_integer(*reg)?;
            Ok(regst)
        })
}
pub fn get_operation_mode(modem: &mut HuaweiModem) -> impl Future<Item = ModemOperationMode, Error = HuaweiError> {
    modem.send_raw(AtCommand::Read { param: "+CFUN".into() })
        .and_then(|pkt| {
            let rpl = pkt.extract_named_response("+CFUN")?
                .get_integer()?;
            Ok(ModemOperationMode::from_integer(*rpl)?)
        })
}
pub fn get_pin_state(modem: &mut HuaweiModem) -> impl Future<Item = PinState, Error = HuaweiError> {
    modem.send_raw(AtCommand::Read { param: "+CPIN".into() })
        .and_then(|pkt| {
            let rpl = pkt.extract_named_response("+CPIN")?
                .get_unknown()?;
            Ok(PinState::from_string(rpl)?)
        })
}
pub fn input_pin(modem: &mut HuaweiModem, pin: String) -> impl Future<Item = (), Error = HuaweiError> {
    modem.send_raw(AtCommand::Equals { 
        param: "+CPIN".into(),
        value: AtValue::String(pin)
    }).and_then(|pkt| {
        pkt.assert_ok()?;
        Ok(())
    })
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SignalQuality {
    pub rssi: u32,
    pub ber: u32
}
pub fn get_signal_quality(modem: &mut HuaweiModem) -> impl Future<Item = SignalQuality, Error = HuaweiError> {
    modem.send_raw(AtCommand::Execute { command: "+CSQ".into() })
        .and_then(|pkt| {
            let rpl = pkt.extract_named_response("+CSQ")?
                .get_array()?;
            let rssi = rpl.get(0)
                .ok_or(HuaweiError::TypeMismatch)?
                .get_integer()?;
            let ber = rpl.get(1)
                .ok_or(HuaweiError::TypeMismatch)?
                .get_integer()?;
            Ok(SignalQuality { rssi: *rssi, ber: *ber })
        })
}
