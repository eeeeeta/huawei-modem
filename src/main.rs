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
extern crate num;
#[macro_use] extern crate num_derive;

use std::fs::{File, OpenOptions};
use tokio_file_unix::File as FileNb;
use codec::AtCodec;
use at::{AtCommand};
use futures::{Future, Poll};
use tokio_io::AsyncRead;
use futures::sync::{oneshot, mpsc};
use future::{ModemRequest, ModemResponse, HuaweiModemFuture};
use tokio_core::reactor::{Core, Handle};
use pdu::{HexData, Pdu};
use std::io::prelude::*;
pub use errors::HuaweiResult;
pub type HuaweiFuture<T> = Box<Future<Item = T, Error = errors::HuaweiError>>;

pub mod error_codes;
pub mod errors;
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
}
fn main() {
    env_logger::init().unwrap();
    let mut core = Core::new().unwrap();
    let mut modem = HuaweiModem::new_from_path("/dev/ttyUSB0", &core.handle()).unwrap();
    println!("Input data in the form smsc;recipient;message");
    let stdin = ::std::io::stdin();
    let lock = stdin.lock();
    for ln in lock.lines() {
        let ln = ln.unwrap();
        let ln = ln.split(";").collect::<Vec<_>>();
        println!("Sending \"{}\" to {} (SMSC {})...", ln[2], ln[1], ln[0]);
        let msg = Pdu::make_simple_message(ln[0], ln[1], ln[2]);
        println!("{:?}", msg);
        println!("{}", HexData(&msg.as_bytes().0));
        let fut = cmd::sms::send_sms_pdu(&mut modem, &msg);
        println!("Result: {:?}", core.run(fut));
    }
}
