//! Functions for dealing with registration on a GSM network (signal quality, PIN, etc.)
use crate::{HuaweiModem};
use crate::at::*;
use crate::errors::*;
use futures::Future;
use crate::util::HuaweiFromPrimitive;

/// The current registration state of the modem (from `AT+CREG`).
/// 
/// Modems have to be 'registered' (i.e. connected to) a given cellular network to be able to do
/// anything useful (text, call, etc.). Therefore, checking the registration state can be useful to
/// figure out why your modem isn't working.
#[repr(u8)]
#[derive(Fail, Debug, FromPrimitive, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegistrationState {
    /// Not registered, and not searching for a new operator at present.
    #[fail(display = "Not registered; not searching for a new operator")]
    NotRegisteredAndDisabled = 0,
    /// Registered, and on our 'home' network (i.e. not roaming)
    #[fail(display = "Registered; on home network")]
    RegisteredHome = 1,
    /// Not registered, and searching for a new operator.
    #[fail(display = "Not registered; searching for a new operator")]
    NotRegisteredSearching = 2,
    /// Registration denied.
    #[fail(display = "Registration denied")]
    RegistrationDenied = 3,
    /// Registration state unknown.
    #[fail(display = "Unknown registration state")]
    Unknown = 4,
    /// Reigstered, and on a 'roaming' network.
    #[fail(display = "Registered; roaming")]
    RegisteredRoaming = 5
}
impl RegistrationState {
    /// If the `RegistrationState` is either `RegisteredHome` or `RegisteredRoaming`, returns
    /// `true`. Otherwise, returns `false`.
    pub fn is_registered(&self) -> bool {
        use self::RegistrationState::*;

        match *self {
            RegisteredHome => true,
            RegisteredRoaming => true,
            _ => false
        }
    }
}
/// The current modem operation mode (from `AT+CFUN`).
///
/// Presumably, this is useful for power saving or something. Note that not all state transitions
/// are necessarily allowed by the modem - in particular, it looks like going from offline to
/// online is not allowed on some modems, presumably requiring a restart. Consulting your modem
/// manual may be advisable.
#[repr(u8)]
#[derive(Fail, Debug, FromPrimitive, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModemOperationMode {
    /// Minimum functionality possible.
    ///
    /// In this mode, RF is disabled, but the SIM card is still powered.
    #[fail(display = "Minimum functionality; disable RF but keep SIM power")]
    MinimumFunctionality = 0,
    /// Online mode.
    #[fail(display = "Online mode")]
    OnlineMode = 1,
    /// Offline mode.
    #[fail(display = "Offline mode")]
    OfflineMode = 4,
    /// FTM mode.
    ///
    /// I have zero clue what this is.
    #[fail(display = "FTM mode")]
    FtmMode = 5,
    /// Restart the modem.
    #[fail(display = "Restart modem")]
    Restart = 6,
    /// Disable RF.
    #[fail(display = "Disable RF")]
    DisableRf = 7
}
/// The PIN state of the modem (from `AT+CPIN`).
///
/// If the SIM is locked with a PIN, you must enter it before using the modem.
#[repr(u8)]
#[derive(Fail, Debug, Copy, Clone, PartialEq, Eq)]
pub enum PinState {
    /// Ready - not pending for any password.
    #[fail(display = "Ready; no passwords required")]
    Ready,
    /// Waiting for a SIM PIN to be entered.
    #[fail(display = "SIM PIN required")]
    SimPin,
    /// Waiting for a SIM PUK to be given (i.e. the SIM PIN is blocked)
    #[fail(display = "SIM PUK required")]
    SimPuk,
    /// Waiting for a SIM PIN2 to be entered.
    #[fail(display = "SIM PIN2 required")]
    SimPin2,
    /// Waiting for a SIM PUK2 to be given (i.e. the SIM PIN2 is blocked)
    #[fail(display = "SIM PUK2 required")]
    SimPuk2
}
impl PinState {
    // FIXME: `crate` because this should ideally use `TryFrom` if public
    pub(crate) fn from_string(st: &str) -> HuaweiResult<Self> {
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
/// Get the modem's current registration state (`AT+CREG`).
pub fn get_registration(modem: &mut HuaweiModem) -> impl Future<Item = RegistrationState, Error = HuaweiError> {
    modem.send_raw(AtCommand::Read { param: "+CREG".into() })
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
/// Get the modem's current operation mode (`AT+CFUN`).
pub fn get_operation_mode(modem: &mut HuaweiModem) -> impl Future<Item = ModemOperationMode, Error = HuaweiError> {
    modem.send_raw(AtCommand::Read { param: "+CFUN".into() })
        .and_then(|pkt| {
            let rpl = pkt.extract_named_response("+CFUN")?
                .get_integer()?;
            Ok(ModemOperationMode::from_integer(*rpl)?)
        })
}
/// Get the modem's current PIN state (`AT+CPIN`).
pub fn get_pin_state(modem: &mut HuaweiModem) -> impl Future<Item = PinState, Error = HuaweiError> {
    modem.send_raw(AtCommand::Read { param: "+CPIN".into() })
        .and_then(|pkt| {
            let rpl = pkt.extract_named_response("+CPIN")?
                .get_unknown()?;
            Ok(PinState::from_string(rpl)?)
        })
}
/// Input the given `pin`, in order to unlock a locked PIN.
pub fn input_pin(modem: &mut HuaweiModem, pin: String) -> impl Future<Item = (), Error = HuaweiError> {
    modem.send_raw(AtCommand::Equals { 
        param: "+CPIN".into(),
        value: AtValue::String(pin)
    }).and_then(|pkt| {
        pkt.assert_ok()?;
        Ok(())
    })
}
/// Signal quality, as returned from the modem (`AT+CSQ`).
///
/// The exact values of this `struct` may vary based on your modem type. Consult your modem manual
/// for more information.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SignalQuality {
    /// Recieved Signal Strength Indication (RSSI) value.
    ///
    /// At least on some modems, this value may have the following meanings:
    ///
    /// - 0 → less than or equal to -113 dBm 
    /// - 1 → -111 dBm
    /// - 2-30 → -109 to -53 dBm
    /// - 31 → greater than or equal to -51 dBm
    /// - 99 → unknown or undetectable.
    pub rssi: u32,
    /// Channel bit error rate, in percent.
    ///
    /// On some modems, this is permanently 99 (i.e. unsupported).
    pub ber: u32
}
/// Get the modem's current signal quality (`AT+CSQ`).
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
