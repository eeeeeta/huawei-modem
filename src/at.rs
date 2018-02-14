//! Types for dealing with AT commands and replies.
use error_codes::CmsError;
use std::fmt;
use errors::{HuaweiError, HuaweiResult};
/// An AT result code, which indicates the completion of a command.
#[derive(Fail, Debug, Clone, PartialEq, Eq, is_enum_variant)]
pub enum AtResultCode {
    /// Command executed without failure.
    #[fail(display = "A command is executed, and there is no error.")]
    Ok,
    /// Connection established.
    #[fail(display = "A connection is established.")]
    Connect,
    /// Incoming call.
    #[fail(display = "An incoming call is originated.")]
    Ring,
    /// Connection terminated.
    #[fail(display = "A connection is terminated.")]
    NoCarrier,
    /// Generic error (rather unhelpful).
    #[fail(display = "A generic error occurred.")]
    Error,
    /// CME error (= generic error), with annoyingly opaque error code (will be fixed).
    ///
    /// There is a list of CME errors that I should really get around to
    /// making into an `enum`. However, that's annoying, so I haven't done
    /// it yet.
    #[fail(display = "An error occurred: code {}", _0)]
    CmeError(u32),
    /// Typed CMS error (= SMS-related error) that uses one of the
    /// available error codes.
    #[fail(display = "An SMS-related error occurred: {}", _0)]
    CmsError(#[cause] CmsError),
    /// CMS error given as string, because of modem configuration.
    ///
    /// There's probably some way to get modems to report errors as a numeric
    /// error code, so you can make use of the `enum`. However, I don't know
    /// of one.
    #[fail(display = "An unknown SMS-related error occurred: {}", _0)]
    CmsErrorString(String),
    /// Unknown CMS error code.
    #[fail(display = "An unknown SMS-related error occurred: code {}", _0)]
    CmsErrorUnknown(u32),
    /// No dialtone.
    #[fail(display = "There is no dialtone.")]
    NoDialtone,
    /// Recipient busy.
    #[fail(display = "Recipient is busy.")]
    Busy,
    /// No answer.
    #[fail(display = "No reply (timeout occurred).")]
    NoAnswer,
    /// Command not supported.
    #[fail(display = "Command not supported.")]
    CommandNotSupported,
    /// Too many parameters.
    #[fail(display = "Too many parameters.")]
    TooManyParameters
}
/// Any of a set of types used in AT commands.
#[derive(Debug, Clone, PartialEq, Eq, is_enum_variant)]
pub enum AtValue {
    /// A string-type value - text surrounded by "quotation marks".
    String(String),
    /// An integer.
    Integer(u32),
    /// A range of integers.
    Range((u32, u32)),
    /// Some untyped value - usually 'bareword' strings, i.e. strings that aren't
    /// surrounded in "quotation marks".
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
        /// This `impl` block provides methods to extract various types
        /// out of an `AtValue`. If the value is not of the desired type,
        /// `HuaweiError::TypeMismatch` is returned.
        ///
        /// - `as_x` methods take `self`, and return either the type or an error.
        /// - `get_x` methods take `&self`, and return a `&` reference.
        /// - `get_x_mut` methods take `&mut self`, and return a `&mut` reference.
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
/// Writes the `AtValue` out, as it would appear on the command line.
///
/// This `impl` is directly used for formatting `AtValue`s when making
/// AT commands.
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
/// One of possibly many response lines to an AT command.
///
/// One `AtResponse` always corresponds to one line of text.
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
/// The complete set of responses to an issued AT command.
#[derive(Debug, Clone)]
pub struct AtResponsePacket {
    /// The various `AtResponses` issued.
    ///
    /// Note that this will only contain 'expected' `InformationResponse`s,
    /// as well as any `Unknown` responses. 'Expected' values are values
    /// that were expected as a result of the command issued - for more
    /// information, see the `AtCommand` documentation.
    pub responses: Vec<AtResponse>,
    /// The final result code for this command.
    pub status: AtResultCode
}
impl AtResponsePacket {
    /// Extracts the value of an `InformationResponse` that has a given `resp`
    /// as its `param`, if such a response exists.
    ///
    /// Also invokes `self.assert_ok()?`, to verify that the response was successful.
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
    /// Like `extract_named_response_opt`, but fails with a `HuaweiError::ExpectedResponse` if the
    /// named response doesn't actually exist.
    pub fn extract_named_response(&self, resp: &str) -> HuaweiResult<&AtValue> {
        match self.extract_named_response_opt(resp)? {
            Some(val) => Ok(val),
            None => Err(HuaweiError::ExpectedResponse(resp.into()))
        }
    }
    /// Returns `HuaweiError::AtError(self.status.clone())` if the status code was not `Ok`.
    pub fn assert_ok(&self) -> HuaweiResult<()> {
        if self.status.is_ok() {
            Ok(())
        }
        else {
            Err(HuaweiError::AtError(self.status.clone()))
        }
    }
}
impl AtCommand {
    /// Get the set of 'expected' `InformationResponse`s for this command.
    ///
    /// This is used by the library to filter out URCs (Unsolicited Response Codes) - basically,
    /// commands only get `InformationResponse`s that match their `expected()` array, so we can
    /// filter all of the other responses out and assume that they're URCs.
    ///
    /// - For `Equals`, `Read`, and `Test`, this is the value of `vec![param]`.
    /// - For `Execute` and `Basic`, this is the value of `vec![command]`.
    /// - For `Text`, this is the value of `expected`.
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
/// An AT command.
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
        /// The set of 'expected' `InformationResponse`s to this command.
        expected: Vec<String>
    }
}
/// Writes the `AtCommand` out, as it would appear on the command line.
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
