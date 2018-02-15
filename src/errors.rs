//! Error handling.
use std::io;
use futures::sync::oneshot::Canceled;
use pdu::MessageEncoding;
use at;
use failure::Fail;

/// An error either raised by a command implementation, or by the library itself.
#[derive(Fail, Debug)]
pub enum CommandError<T> where T: Fail {
    /// An error in the implementation of some generic command.
    #[fail(display = "Error in command: {}", _0)]
    Command(#[cause] T),
    /// An error raised by the library itself.
    #[fail(display = "{}", _0)]
    Huawei(#[cause] HuaweiError)
}
impl<T> From<HuaweiError> for CommandError<T> where T: Fail {
    fn from(e: HuaweiError) -> CommandError<T> {
        CommandError::Huawei(e)
    }
}
/// Error `enum` for errors raised by this library.
///
/// Exhaustive matching is NOT guaranteed by the library API (!).
#[derive(Fail, Debug)]
pub enum HuaweiError {
    /// The background future used to talk to the modem died, making any sort of interaction with
    /// any library feature somewhat...difficult.
    #[fail(display = "Failed to communicate with the background future (it's likely dead).")]
    FutureDied,
    /// An error from the modem itself.
    #[fail(display = "Error from modem: {}", _0)]
    AtError(#[cause] at::AtResultCode),
    /// Some random I/O error.
    #[fail(display = "An I/O error occurred: {}", _0)]
    IoError(#[cause] io::Error),
    /// An error parsing data from the modem.
    #[fail(display = "There was an error parsing data.")]
    ParseError(::nom::ErrorKind),
    /// An indication that an `InformationResponse` of some form from the modem was expected, but
    /// never provided.
    #[fail(display = "Expected a {} response", _0)]
    ExpectedResponse(String),
    /// A type mismatch occured when parsing data from the modem.
    #[fail(display = "Type mismatch when parsing reply")]
    TypeMismatch,
    /// A value provided by the modem was out of range.
    #[fail(display = "Value out of range: {}", _0)]
    ValueOutOfRange(at::AtValue),
    /// An error occured parsing a PDU.
    #[fail(display = "Invalid PDU: {}", _0)]
    InvalidPdu(&'static str),
    /// Unsupported user data encoding. The raw bytes are provided for your edification.
    #[fail(display = "Data of unknown encoding {:?}: {:?}", _0, _1)]
    UnsupportedEncoding(MessageEncoding, Vec<u8>),
    /// This shouldn't be shown, and is designed to stop you matching on this `enum` exhaustively.
    /// If you do that, yo' code gonna break!
    #[fail(display = "[this should never be shown]")]
    #[doc(hidden)]
    __Nonexhaustive
}
impl From<io::Error> for HuaweiError {
    fn from(e: io::Error) -> HuaweiError {
        HuaweiError::IoError(e)
    }
}
impl From<::nom::ErrorKind> for HuaweiError {
    fn from(e: ::nom::ErrorKind) -> HuaweiError {
        HuaweiError::ParseError(e)
    }
}
impl From<Canceled> for HuaweiError {
    fn from(_: Canceled) -> HuaweiError {
        HuaweiError::FutureDied
    }
}
/// Bog-standard result type alias.
pub type HuaweiResult<T> = Result<T, HuaweiError>;
