use crate::at::*;
use crate::error_codes::CmsError;
use num::FromPrimitive;
use encoding::{Encoding, DecoderTrap};
use encoding::all::{ASCII};
use nom::{line_ending, not_line_ending};
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};
named!(pub parse_string(&[u8]) -> String,
       map_res!(
           delimited!(
               tag!("\""),
               take_until!("\""),
               tag!("\"")
           ),
           |data| {
               ASCII.decode(data, DecoderTrap::Replace)
           }
       )
);
named!(pub parse_ucs2_string(&[u8]) -> String,
       map!(
           delimited!(
               tag!("\""),
               many0!(
                   map_res!(
                       count!(
                           one_of!("0123456789ABCDEF"),
                           4
                       ),
                       |data: Vec<char>| {
                           let st: String = data.into_iter().collect();
                           u16::from_str_radix(&st, 16)
                       }
                   )
               ),
               tag!("\"")
           ),
           |data: Vec<u16>| {
               decode_utf16(data.into_iter())
                   .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
                   .collect::<String>()
           }
       )
);
named!(pub parse_integer(&[u8]) -> u32,
       map_res!(
           many1!(
               one_of!("0123456789")
           ),
           |data: Vec<char>| {
               let st: String = data.into_iter().collect();
               st.parse()
           }
       )
);
named!(pub parse_range(&[u8]) -> (u32, u32),
       do_parse!(
           i1: parse_integer >>
           tag!("-") >>
           i2: parse_integer >>
               (i1, i2)
       )
);
named!(pub parse_unknown(&[u8]) -> String,
       map!(
           many1!(none_of!(",")),
           |data| {
               data.into_iter().collect()
           }
       )
);
named!(pub parse_value(&[u8]) -> AtValue,
       map!(
           do_parse!(
               first: parse_single_value >>
               others: many0!(
                   preceded!(
                       tag!(","),
                       parse_single_value
                   )
               ) >>
               (first, others)
           ),
           |(first, others)| {
               if others.len() == 0 {
                   first
               }
               else {
                   let mut ret = vec![first];
                   ret.extend(others);
                   AtValue::Array(ret)
               }
           }
       )
);
named!(pub parse_bracketed_array(&[u8]) -> AtValue,
       map!(
           delimited!(
               tag!("("),
               flat_map!(take_until!(")"), parse_value),
               tag!(")")
           ),
           |v| {
               match v {
                   AtValue::Array(ret) => AtValue::BracketedArray(ret),
                   AtValue::Empty => AtValue::BracketedArray(vec![]),
                   x => AtValue::BracketedArray(vec![x])
               }
           }
       )
);
named!(pub parse_empty(&[u8]) -> (),
       value!(())
);
named!(pub parse_single_value(&[u8]) -> AtValue,
       alt_complete!(
           parse_bracketed_array |
           map!(parse_string, |s| AtValue::String(s.into())) |
           map!(parse_range, |x| AtValue::Range(x)) |
           map!(parse_integer, |i| AtValue::Integer(i)) |
           map!(parse_unknown, |u| AtValue::Unknown(u.into())) |
           map!(parse_empty, |_| AtValue::Empty)
       )
);
named!(pub parse_information_response(&[u8]) -> (String, AtValue),
       map!(
           do_parse!(
               param: take_until_s!(":") >>
               tag!(":") >>
               opt!(tag!(" ")) >>
               response: parse_value >>
               (param, response)
           ),
           |(param, response)| (::std::str::from_utf8(param).unwrap().into(), response)
       )
);
named!(pub parse_response_code(&[u8]) -> AtResultCode,
       alt_complete!(
           map!(tag!("OK"), |_| AtResultCode::Ok) |
           map!(tag!("CONNECT"), |_| AtResultCode::Connect) |
           map!(tag!("RING"), |_| AtResultCode::Ring) |
           map!(tag!("NO CARRIER"), |_| AtResultCode::NoCarrier) |
           map!(tag!("ERROR"), |_| AtResultCode::Error) |
           map!(tag!("NO DIALTONE"), |_| AtResultCode::NoDialtone) |
           map!(tag!("BUSY"), |_| AtResultCode::Busy) |
           map!(tag!("NO ANSWER"), |_| AtResultCode::NoAnswer) |
           map!(tag!("COMMAND NOT SUPPORT"), |_| AtResultCode::CommandNotSupported) |
           map!(tag!("TOO MANY PARAMETERS"), |_| AtResultCode::TooManyParameters) |
           map_res!(parse_information_response, |(p, r)| {
               if p == "+CME ERROR" {
                   if let AtValue::Integer(r) = r {
                       return Ok(AtResultCode::CmeError(r));
                   }
               }
               if p == "+CMS ERROR" {
                   if let AtValue::Integer(r) = r {
                       if let Some(e) = CmsError::from_u32(r) {
                           return Ok(AtResultCode::CmsError(e));
                       }
                       else {
                           return Ok(AtResultCode::CmsErrorUnknown(r));
                       }
                   }
                   else if let AtValue::Unknown(s) = r {
                       return Ok(AtResultCode::CmsErrorString(s));
                   }
               }
               Err("Incorrect information response")
           })
       )
);
named!(pub parse_response_line(&[u8]) -> AtResponse,
       alt_complete!(
           map!(parse_response_code, |c| AtResponse::ResultCode(c)) |
           map!(parse_information_response, |(p, r)| AtResponse::InformationResponse {
               param: p.into(),
               response: r
           }) |
           map_res!(not_line_ending, |s| {
               let st = ::std::str::from_utf8(s).map_err(|_| ())?.trim();
               if st.len() == 0 {
                   return Err(());
               }
               Ok(AtResponse::Unknown(st.to_string().into()))
           })
       )
);
named!(pub responses(&[u8]) -> Vec<AtResponse>,
       map!(
           many1!(
               terminated!(
                   opt!(flat_map!(not_line_ending, parse_response_line)),
                   line_ending
               )
           ),
           |res| {
               res.into_iter().filter_map(|s| s).collect()
           }
       )
);
#[cfg(test)]
mod test {
    use super::*;
    use crate::at::AtValue::*;
    #[test]
    fn value_string() {
        assert_eq!(parse_string(b"\"testing\"").unwrap(),
                   (&[] as &[_], "testing".into()));
        assert_eq!(parse_value(b"\"testing\"").unwrap(),
                   (&[] as &[_], AtValue::String("testing".into())));
    }
    #[test]
    fn value_integer() {
        assert_eq!(parse_integer(b"9001").unwrap(),
                   (&[] as &[_], 9001));
        assert_eq!(parse_value(b"9001").unwrap(),
                   (&[] as &[_], AtValue::Integer(9001)));
    }
    #[test]
    fn value_range() {
        assert_eq!(parse_range(b"2-9001").unwrap(),
                   (&[] as &[_], (2, 9001)));
        assert_eq!(parse_value(b"2-9001").unwrap(),
                   (&[] as &[_], AtValue::Range((2, 9001))));
    }
    #[test]
    fn value_empty() {
        assert_eq!(parse_empty(b"").unwrap(),
                   (&[] as &[_], ()));
        assert_eq!(parse_value(b"").unwrap(),
                   (&[] as &[_], AtValue::Empty));
    }
    #[test]
    fn value_unknown() {
        assert_eq!(parse_unknown(b"invalid").unwrap(),
                   (&[] as &[_], "invalid".into()));
        assert_eq!(parse_value(b"invalid").unwrap(),
                   (&[] as &[_], AtValue::Unknown("invalid".into())));
    }
    #[test]
    fn value_complex() {
        assert_eq!(
            parse_value(b"3,0,15,\"GSM\",(),(0-3),,(0-1),invalid,(0-2,15),(\"GSM\",\"IRA\")").unwrap(),
            (&[] as &[_], Array(vec![
                Integer(3),
                Integer(0),
                Integer(15),
                String("GSM".into()),
                BracketedArray(vec![]),
                BracketedArray(vec![
                    Range((0, 3))
                ]),
                Empty,
                BracketedArray(vec![
                    Range((0, 1))
                ]),
                Unknown("invalid".into()),
                BracketedArray(vec![
                    Range((0, 2)),
                    Integer(15)
                ]),
                BracketedArray(vec![
                    String("GSM".into()),
                    String("IRA".into()),
                ])
            ]))
        )
    }
}
