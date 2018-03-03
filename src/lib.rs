#![feature(conservative_impl_trait, try_from)]

extern crate futures;
extern crate tokio_core;
extern crate tokio_file_unix;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate failure;
#[macro_use] extern crate failure_derive;
#[macro_use] extern crate nom;
extern crate encoding;
extern crate tokio_io;
extern crate bytes;
#[macro_use] extern crate derive_is_enum_variant;
extern crate num;
#[macro_use] extern crate num_derive;
extern crate rand;

use std::fs::{File, OpenOptions};
use tokio_file_unix::File as FileNb;
use codec::AtCodec;
use at::{AtResponse, AtCommand};
use futures::{Future, Poll};
use tokio_io::AsyncRead;
use futures::sync::{oneshot, mpsc};
use future::{ModemRequest, ModemResponse, HuaweiModemFuture};
use tokio_core::reactor::Handle;
pub use errors::HuaweiResult;
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
use errors::HuaweiError;

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
        let framed = ev.framed(AtCodec);
        let (tx, rx) = mpsc::unbounded();
        let (urctx, urcrx) = mpsc::unbounded();
        let fut = HuaweiModemFuture::new(framed, rx, urctx);
        h.spawn(fut.map_err(|e| {
            error!("HuaweiModemFuture failed: {}", e);
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

