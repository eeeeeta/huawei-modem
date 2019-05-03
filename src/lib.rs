//! The `huawei-modem` library provides a set of utilities for interfacing with USB 3G/HSDPA/UMTS
//! modems (particularly Huawei models, like the E220 and E3531) that use the Hayes/AT command set.
//!
//! At present, the library's main consumer is
//! [sms-irc](https://git.theta.eu.org/sms-irc.git/about/). In particular, it may be helpful to
//! look at [modem.rs](https://git.theta.eu.org/sms-irc.git/tree/src/modem.rs) inside that project
//! to get a feel for how to use this library, as well as looking inside the `examples/`
//! subdirectory to see some simple SMS sending/receiving examples.

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

/// Bog-standard boxed future alias.
pub type HuaweiFuture<T> = Box<dyn Future<Item = T, Error = errors::HuaweiError>>;

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
mod parse;
pub mod codec;
pub mod cmd;
mod util;
mod future;

use std::path::Path;
use crate::errors::HuaweiError;

/// Future representing a response from the modem.
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
/// A connection to an AT/Huawei-style modem.
pub struct HuaweiModem {
    tx: mpsc::UnboundedSender<ModemRequest>,
    urc: Option<mpsc::UnboundedReceiver<AtResponse>>
}
impl HuaweiModem {
    /// Start talking to the modem at a specified file path.
    pub fn new_from_path<P: AsRef<Path>>(path: P, h: &Handle) -> HuaweiResult<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;
        Self::new_from_file(file, h)
    }
    /// Start talking to the modem represented by a given file handle.
    ///
    /// The file handle provided must support non-blocking IO for this method to work.
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
    /// Retrieve the URC (Unsolicited Result Code) receiver from the modem (it can only be taken
    /// once).
    ///
    /// This gives you an `UnboundedReceiver` that provides you with a stream of `AtResponse`s that
    /// are *unsolicited*, i.e. they are proactive notifications from the modem of something
    /// happening. On some modems, you may well receive a steady stream of random updates.
    ///
    /// This can be useful when you configure your modem for message notification on delivery (see
    /// `cmd::sms::set_new_message_indications`), in which case you'll want to check for `CNMI`
    /// URCs through this receiver and use that to poll for new messages.
    pub fn take_urc_rx(&mut self) -> Option<mpsc::UnboundedReceiver<AtResponse>> {
        self.urc.take()
    }
    /// Send a raw AT command to the modem.
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

