#[macro_use] extern crate log;
#[macro_use] extern crate failure_derive;
#[macro_use] extern crate nom;
#[macro_use] extern crate derive_is_enum_variant;
#[macro_use] extern crate num_derive;

use std::fs::{File, OpenOptions};
use tokio_file_unix::File as FileNb;
use crate::codec::AtCodec;
use crate::at::{AtResponse, AtCommand};
use futures::{Future, Poll};
use futures::sync::{oneshot, mpsc};
use crate::future::{ModemRequest, ModemResponse, HuaweiModemFuture};
use tokio_core::reactor::Handle;
use tokio_codec::Decoder;
pub use crate::errors::HuaweiResult;
pub type HuaweiFuture<T> = Box<Future<Item = T, Error = errors::HuaweiError>>;

macro_rules! check_offset {
    ($b:ident, $offset:ident, $reason:expr) => {
        if $b.get($offset).is_none() {
            return Err(HuaweiError::InvalidPdu(concat!("Offset check failed for: ", $reason)));
        }
    }
}

pub mod error_codes;
pub mod errors;
pub mod gsm_encoding;
pub mod at;
pub mod pdu;
pub mod parse;
pub mod codec;
pub mod cmd;
pub mod util;
mod future;

use std::path::Path;
use crate::errors::HuaweiError;

pub struct ModemResponseFuture {
    rx: Result<oneshot::Receiver<ModemResponse>, ()>
}
impl Future for ModemResponseFuture {
    type Item = ModemResponse;
    type Error = HuaweiError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.rx {
            Ok(ref mut rx) => Ok(rx.poll()?),
            Err(_) => Err(HuaweiError::FutureDied)
        }
    }
}
pub struct HuaweiModem {
    tx: mpsc::UnboundedSender<ModemRequest>,
    urc: Option<mpsc::UnboundedReceiver<AtResponse>>
}
impl HuaweiModem {
    pub fn new_from_path<P: AsRef<Path>>(path: P, h: &Handle) -> HuaweiResult<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;
        Self::new_from_file(file, h)
    }
    pub fn new_from_file(f: File, h: &Handle) -> HuaweiResult<Self> {
        let ev = FileNb::new_nb(f)?.into_io(h)?;
        let framed = AtCodec.framed(ev);
        let (tx, rx) = mpsc::unbounded();
        let (urctx, urcrx) = mpsc::unbounded();
        let fut = HuaweiModemFuture::new(framed, rx, urctx);
        h.spawn(fut.map_err(|e| {
            error!("HuaweiModemFuture failed: {}", e);
            error!("Backtrace: {}", e.backtrace());
            ()
        }));
        Ok(Self { tx, urc: Some(urcrx) })
    }
    pub fn take_urc_rx(&mut self) -> Option<mpsc::UnboundedReceiver<AtResponse>> {
        self.urc.take()
    }
    pub fn send_raw(&mut self, cmd: AtCommand) -> ModemResponseFuture {
        let (tx, rx) = oneshot::channel();
        let expected = cmd.expected();
        let req = ModemRequest {
            command: cmd,
            notif: tx,
            expected
        };
        if let Err(_) = self.tx.unbounded_send(req) {
            ModemResponseFuture { rx: Err(()) }
        }
        else {
            ModemResponseFuture { rx: Ok(rx) }
        }
    }
}

