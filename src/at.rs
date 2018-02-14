use error_codes::CmsError;
use std::fmt;
use errors::{HuaweiError, HuaweiResult};
#[derive(Fail, Debug, Clone, PartialEq, Copy, Eq, is_enum_variant)]
pub enum AtResultCode {
    #[fail(display = "A command is executed, and there is no error.")]
    Ok,
    #[fail(display = "A connection is established.")]
    Connect,
    #[fail(display = "An incoming call is originated.")]
    Ring,
    #[fail(display = "A connection is terminated.")]
    NoCarrier,
    #[fail(display = "A generic error occurred.")]
    Error,
    #[fail(display = "An error occurred: code {}", _0)]
    CmeError(u32),
    #[fail(display = "An SMS-related error occurred: {}", _0)]
    CmsError(#[cause] CmsError),
    #[fail(display = "An unknown SMS-related error occurred: code {}", _0)]
    CmsErrorUnknown(u32),
    #[fail(display = "There is no dialtone.")]
    NoDialtone,
    #[fail(display = "Recipient is busy.")]
    Busy,
    #[fail(display = "No reply (timeout occurred).")]
    NoAnswer,
    #[fail(display = "Command not supported.")]
    CommandNotSupported,
    #[fail(display = "Too many parameters.")]
    TooManyParameters
}
#[derive(Debug, Clone, PartialEq, Eq, is_enum_variant)]
pub enum AtValue {
    /// A string-type value - text surrounded by "quotation marks".
    String(String),
    /// An integer.
    Integer(u32),
    /// A range of integers.
    Range((u32, u32)),
    /// Some value of unknown type.
    Unknown(String),
    /// An empty value, corresponding to nothing at all.
    Empty,
    /// A bracketed array.
    BracketedArray(Vec<AtValue>),
    /// A non-bracketed array.
    Array(Vec<AtValue>)
}
macro_rules! at_value_impl {
    ($atv:ident, $($var:ident, $refmeth:ident, $mutmeth:ident, $asmeth:ident, $ty:ty),*) => {
        impl $atv {
            $(
                pub fn $refmeth(&self) -> HuaweiResult<&$ty> {
                    if let $atv::$var(ref i) = *self {
                        Ok(i)
                    }
                    else {
                        Err(HuaweiError::TypeMismatch)
                    }
                }
                pub fn $mutmeth(&mut self) -> HuaweiResult<&mut $ty> {
                    if let $atv::$var(ref mut i) = *self {
                        Ok(i)
                    }
                    else {
                        Err(HuaweiError::TypeMismatch)
                    }
                }
                pub fn $asmeth(self) -> HuaweiResult<$ty> {
                    if let $atv::$var(i) = self {
                        Ok(i)
                    }
                    else {
                        Err(HuaweiError::TypeMismatch)
                    }
                }
             )*
        }
    }
}
at_value_impl!(AtValue,
               String, get_string, get_string_mut, as_string, String,
               Integer, get_integer, get_integer_mut, as_integer, u32,
               Range, get_range, get_range_mut, as_range, (u32, u32),
               Unknown, get_unknown, get_unknown_mut, as_unknown, String,
               BracketedArray, get_bracketed_array, get_bracketed_array_mut, as_bracketed_array, Vec<AtValue>,
               Array, get_array, get_array_mut, as_array, Vec<AtValue>);
impl fmt::Display for AtValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::AtValue::*;
        match *self {
            String(ref st) => write!(f, "\"{}\"", st)?,
            Integer(i) => write!(f, "{}", i)?,
            Range((a, b)) => write!(f, "{}-{}", a, b)?,
            Unknown(ref st) => write!(f, "{}", st)?,
            Empty => {},
            BracketedArray(ref val) => {
                write!(f, "(")?;
                for (i, val) in val.iter().enumerate() {
                    let c = if i == 0 { "" } else { "," };
                    write!(f, "{}{}", c, val)?;
                }
                write!(f, ")")?;
            },
            Array(ref val) => {
                for (i, val) in val.iter().enumerate() {
                    let c = if i == 0 { "" } else { "," };
                    write!(f, "{}{}", c, val)?;
                }
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, is_enum_variant)]
pub enum AtResponse {
    /// An information response issued as a result of a command.
    ///
    /// Corresponds to '<param>: <response>'.
    InformationResponse {
        param: String,
        response: AtValue
    },
    /// An AT result code, indicating the completion of a command.
    ResultCode(AtResultCode),
    /// Some other unknown response.
    Unknown(String)
}
#[derive(Debug, Clone)]
pub struct AtResponsePacket {
    pub responses: Vec<AtResponse>,
    pub status: AtResultCode
}
impl AtResponsePacket {
    pub fn extract_named_response_opt(&self, resp: &str) -> HuaweiResult<Option<&AtValue>> {
        self.assert_ok()?;
        for r in self.responses.iter() {
            if let AtResponse::InformationResponse { ref param, ref response } = *r {
                if resp == param {
                    return Ok(Some(response));
                }
            }
        }
        Ok(None)
    }
    pub fn extract_named_response(&self, resp: &str) -> HuaweiResult<&AtValue> {
        match self.extract_named_response_opt(resp)? {
            Some(val) => Ok(val),
            None => Err(HuaweiError::ExpectedResponse(resp.into()))
        }
    }
    pub fn assert_ok(&self) -> HuaweiResult<()> {
        if self.status.is_ok() {
            Ok(())
        }
        else {
            Err(HuaweiError::AtError(self.status))
        }
    }
}
impl AtCommand {
    pub fn expected(&self) -> Vec<String> {
        match *self {
            AtCommand::Equals { ref param, .. } => vec![param.clone()],
            AtCommand::Execute { ref command } => vec![command.clone()],
            AtCommand::Read { ref param } => vec![param.clone()],
            AtCommand::Test { ref param } => vec![param.clone()],
            AtCommand::Basic { ref command, .. } => vec![command.clone()],
            AtCommand::Text { ref expected, .. } => expected.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, is_enum_variant)]
pub enum AtCommand {
    /// Either execute a non-basic command named `param` with `value` as
    /// argument, or set the current value of `param` to `value`.
    ///
    /// Corresponds to `AT<param>=<value>`.
    Equals {
        param: String,
        value: AtValue,
    },
    /// Execute a non-basic command, with the name of `command`.
    ///
    /// Corresponds to `AT<command>`.
    Execute {
        command: String
    },
    /// Read the current value of `param`.
    ///
    /// Corresponds to `AT<param>?`.
    Read {
        param: String
    },
    /// Return the available value range of `param`.
    ///
    /// Corresponds to `AT<param>=?'.
    Test {
        param: String
    },
    /// Execute a basic command, where `command` indicates a single letter (A-Z)
    /// or the & symbol and a single letter, with an optional number parameter.
    ///
    /// Corresponds to `AT<command>[<number>]`.
    Basic {
        command: String,
        number: Option<usize>
    },
    /// Just send some raw text.
    Text {
        text: String,
        expected: Vec<String>
    }
}
impl fmt::Display for AtCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::AtCommand::*;
        match *self {
            Equals { ref param, ref value } => write!(f, "AT{}={}", param, value)?,
            Execute { ref command } => write!(f, "AT{}", command)?,
            Read { ref param } => write!(f, "AT{}?", param)?,
            Test { ref param } => write!(f, "AT{}=?", param)?,
            Basic { ref command, ref number } => {
                write!(f, "AT{}", command)?;
                if let Some(n) = *number {
                    write!(f, "{}", n)?;
                }
            },
            Text { ref text, .. } => write!(f, "{}", text)?
        }
        Ok(())
    }
}
