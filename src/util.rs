use num::FromPrimitive;
use errors::{HuaweiResult, HuaweiError};
use at::AtValue;

pub trait HuaweiFromPrimitive where Self: Sized {
    fn from_integer(i: u32) -> HuaweiResult<Self>;
}
impl<T> HuaweiFromPrimitive for T where T: FromPrimitive {
    fn from_integer(i: u32) -> HuaweiResult<T> {
        if let Some(s) = T::from_u32(i) {
            Ok(s)
        }
        else {
            Err(HuaweiError::ValueOutOfRange(AtValue::Integer(i)))
        }
    }
}
