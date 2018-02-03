use std::io;
use futures::sync::oneshot::Canceled;
use failure::Fail;
#[derive(Fail, Debug)]
pub enum ExecuteError<T> where T: Fail {
    #[fail(display = "Command failure: {}", _0)]
    Command(#[cause] T),
    #[fail(display = "Modem library failure: {}", _0)]
    Huawei(#[cause] HuaweiError)
}
#[derive(Fail, Debug)]
pub enum HuaweiError {
    #[fail(display = "Failed to communicate with the background future (it's likely dead).")]
    FutureDied,
    #[fail(display = "An I/O error occurred: {}", _0)]
    IoError(#[cause] io::Error),
    #[fail(display = "There was an error parsing data.")]
    ParseError(::nom::ErrorKind),
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
