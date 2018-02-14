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
use at::{AtResponse, AtCommand};
use futures::{Future, Poll};
use tokio_io::AsyncRead;
use futures::sync::{oneshot, mpsc};
use future::{ModemRequest, ModemResponse, HuaweiModemFuture};
use tokio_core::reactor::{Core, Handle};
use pdu::{HexData, Pdu, PduAddress, GsmMessageData};
use std::io::prelude::*;
pub use errors::HuaweiResult;
pub type HuaweiFuture<T> = Box<Future<Item = T, Error = errors::HuaweiError>>;

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
fn main() {
    use futures::Stream;

    env_logger::init().unwrap();
    let mut core = Core::new().unwrap();
    let mut modem = HuaweiModem::new_from_path("/dev/ttyUSB2", &core.handle()).unwrap();
    let urcfut = modem.take_urc_rx().unwrap().for_each(|item| {
        println!("URC: {:?}", item);
        Ok(())
    });
    core.handle().spawn(urcfut);
    println!("Setting textmode false...");
    let fut = cmd::sms::set_sms_textmode(&mut modem, false);
    println!("Result: {:?}", core.run(fut));
    println!("Input data in the form recipient;message");
    let stdin = ::std::io::stdin();
    let lock = stdin.lock();
    for ln in lock.lines() {
        let ln = ln.unwrap();
        if ln == "read" {
            println!("Reading messages...");
            let fut = cmd::sms::list_sms_pdu(&mut modem, cmd::sms::MessageStatus::All)
                .map(|v| {
                    for msg in v {
                        println!("Message: {:?}", msg);
                        println!("Text: {}", msg.pdu.get_message_data().decode_message().unwrap_or("[unreadable]".into()));
                    }
                });
            println!("Result: {:?}", core.run(fut));
            continue;
        }
        let ln = ln.split(";").collect::<Vec<_>>();
        println!("Sending \"{}\" to {}...", ln[1], ln[0]);
        let recipient = PduAddress::from_str(ln[0]);
        println!("Recipient: {:?}", recipient);
        let msg = GsmMessageData::encode_message(ln[1]);
        println!("Message data: {:?}", msg);
        let msg = Pdu::make_simple_message(recipient, msg);
        println!("PDU: {:?}", msg);
        let pdu = Pdu::from_bytes(&msg.as_bytes().0).unwrap();
        assert_eq!(pdu, msg);
        println!("Encoded PDU: {}", HexData(&msg.as_bytes().0));
        let fut = cmd::sms::send_sms_pdu(&mut modem, &msg);
        println!("Result: {:?}", core.run(fut));
    }
}
