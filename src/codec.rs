//! Contains the Tokio codec used to decode the AT protocol.
use tokio_io::codec::{Encoder, Decoder};
use bytes::BytesMut;
use at::{AtCommand, AtResponse};
use failure;
use parse;

/// Encodes AT commands into text to be sent to a modem, and decodes its responses into AT
/// responses.
pub struct AtCodec;

impl Decoder for AtCodec {
    type Item = Vec<AtResponse>;
    type Error = failure::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        use nom::IResult;
        trace!("decoding data: {:?}", src);
        let (point, data) = match parse::responses(src) {
            IResult::Done(rest, data) => {
                if data.len() == 0 {
                    return Ok(None);
                }
                (rest.len(), data)
            },
            IResult::Error(e) => return Err(e.into()),
            IResult::Incomplete(_) => return Ok(None)
        };
        let len = src.len().saturating_sub(point);
        src.split_to(len);
        Ok(Some(data))
    }
}
impl Encoder for AtCodec {
    type Item = AtCommand;
    type Error = failure::Error;

    fn encode(&mut self, item: AtCommand, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use std::fmt::Write;
        trace!("sending data: {}", item);
        write!(dst, "\r\n{}\r\n", item)?;
        Ok(())
    }
}
