use std::io;
use futures::sync::oneshot::Canceled;
use at;
use failure::Fail;
#[derive(Fail, Debug)]
pub enum CommandError<T> where T: Fail {
    #[fail(display = "Error in command: {}", _0)]
    Command(#[cause] T),
    #[fail(display = "{}", _0)]
    Huawei(#[cause] HuaweiError)
}
impl<T> From<HuaweiError> for CommandError<T> where T: Fail {
    fn from(e: HuaweiError) -> CommandError<T> {
        CommandError::Huawei(e)
    }
}
#[derive(Fail, Debug)]
pub enum HuaweiError {
    #[fail(display = "Failed to communicate with the background future (it's likely dead).")]
    FutureDied,
    #[fail(display = "Error from modem: {}", _0)]
    AtError(#[cause] at::AtResultCode),
    #[fail(display = "An I/O error occurred: {}", _0)]
    IoError(#[cause] io::Error),
    #[fail(display = "There was an error parsing data.")]
    ParseError(::nom::ErrorKind),
    #[fail(display = "Expected a {} response", _0)]
    ExpectedResponse(String),
    #[fail(display = "Type mismatch when parsing reply")]
    TypeMismatch,
    #[fail(display = "Value out of range: {}", _0)]
    ValueOutOfRange(at::AtValue),
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
pub type HuaweiResult<T> = Result<T, HuaweiError>;
