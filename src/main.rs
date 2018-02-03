#![feature(conservative_impl_trait)]

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

use std::fs::{File, OpenOptions};
use tokio_file_unix::File as FileNb;
use codec::AtCodec;
use at::{AtCommand};
use futures::{Future, Poll};
use tokio_io::AsyncRead;
use futures::sync::{oneshot, mpsc};
use future::{ModemRequest, ModemResponse, HuaweiModemFuture};
use tokio_core::reactor::{Core, Handle};
pub use errors::HuaweiResult;
pub type HuaweiFuture<T> = Box<Future<Item = T, Error = errors::HuaweiError>>;

pub mod error_codes;
pub mod errors;
pub mod at;
pub mod parse;
pub mod codec;
mod future;

use failure::Fail;
use std::path::Path;
use errors::{HuaweiError, ExecuteError};
pub trait ModemCommand {
    type Value;
    type Error: Fail;
    type Future: Future<Item = Self::Value, Error = Self::Error>;
    
    fn get_atcmd(&mut self) -> AtCommand;
    fn process_response(&mut self, r: ModemResponse) -> Self::Future;
}
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
    tx: mpsc::UnboundedSender<ModemRequest>
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
        let fut = HuaweiModemFuture::new(framed, rx);
        h.spawn(fut.map_err(|e| {
            error!("HuaweiModemFuture failed: {}", e);
            ()
        }));
        Ok(Self { tx })
    }
    pub fn send_raw(&mut self, cmd: AtCommand) -> ModemResponseFuture {
        let (tx, rx) = oneshot::channel();
        let req = ModemRequest {
            command: cmd,
            notif: tx
        };
        if let Err(_) = self.tx.unbounded_send(req) {
            ModemResponseFuture { rx: Err(()) }
        }
        else {
            ModemResponseFuture { rx: Ok(rx) }
        }
    }
    pub fn execute_command<C: ModemCommand>(&mut self, mut cmd: C) -> impl Future<Item = C::Value, Error = ExecuteError<C::Error>> {
        let at = cmd.get_atcmd();
        let fut = self.send_raw(at);
        fut.map_err(|e| {
            ExecuteError::Huawei(e)
        }).and_then(move |e| cmd.process_response(e).map_err(|e| {
            ExecuteError::Command(e)
        }))
    }
}
fn main() {
    env_logger::init().unwrap();
    let mut core = Core::new().unwrap();
    let mut modem = HuaweiModem::new_from_path("/dev/ttyUSB0", &core.handle()).unwrap();
    let fut = modem.send_raw(AtCommand::Read {param: "+CREG".into() })
        .and_then(|res| {
            println!("{:?}", res);
            Ok(())
        });
    core.run(fut).unwrap();
}
