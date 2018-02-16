use std::fs::File;
use tokio_file_unix::File as FileNb;
use tokio_core::reactor::PollEvented;
use tokio_io::codec::Framed;
use codec::AtCodec;
use at::{AtResponse, AtResponsePacket, AtCommand};
use futures::{Future, Sink, Stream, Async, Poll};
use futures::sync::{oneshot, mpsc};
use failure;

pub(crate) type ModemResponse = AtResponsePacket;

pub(crate) struct ModemRequest {
    pub(crate) command: AtCommand,
    pub(crate) expected: Vec<String>,
    pub(crate) notif: oneshot::Sender<ModemResponse>
}
struct ModemRequestState {
    notif: oneshot::Sender<ModemResponse>,
    expected: Vec<String>,
    responses: Vec<AtResponse>
}
pub(crate) struct HuaweiModemFuture {
    inner: Framed<PollEvented<FileNb<File>>, AtCodec>,
    rx: mpsc::UnboundedReceiver<ModemRequest>,
    urc: mpsc::UnboundedSender<AtResponse>,
    cur: Option<ModemRequestState>,
    requests: Vec<ModemRequest>,
    fresh: bool,
}
impl HuaweiModemFuture {
    pub(crate) fn new(
        inner: Framed<PollEvented<FileNb<File>>, AtCodec>,
        rx: mpsc::UnboundedReceiver<ModemRequest>,
        urc: mpsc::UnboundedSender<AtResponse>
    ) -> Self {
        Self {
            inner, rx, urc,
            cur: None,
            requests: vec![],
            fresh: true
        }
    }
}
impl Future for HuaweiModemFuture {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        trace!("HuaweiModemFuture woke up");
        if self.fresh {
            self.fresh = false;
            debug!("first poll of future, imposing initial settings");
            let (tx, _) = oneshot::channel();
            let echo = AtCommand::Basic { command: "E".into(), number: Some(0) };
            self.requests.insert(0, ModemRequest {
                command: echo,
                expected: vec![],
                notif: tx
            });
        }
        loop {
            match self.inner.poll()? {
                Async::Ready(r) => {
                    let r = match r {
                        Some(x) => x,
                        None => {
                            debug!("stream ran out, future exiting");
                            return Ok(Async::Ready(()))
                        }
                    };
                    if self.cur.is_some() {
                        if r.iter().any(|x| x.is_result_code()) {
                            let mut state = self.cur.take().unwrap();
                            state.responses.extend(r);
                            debug!("request completed with responses: {:?}", state.responses);
                            let mut resps = vec![];
                            let mut status = None;
                            for resp in state.responses {
                                match resp {
                                    AtResponse::InformationResponse { param, response } => {
                                        if state.expected.contains(&param) {
                                            resps.push(AtResponse::InformationResponse { param, response });
                                        }
                                        else {
                                            self.urc.unbounded_send(AtResponse::InformationResponse { param, response })?;
                                        }
                                    },
                                    AtResponse::ResultCode(x) => {
                                        status = Some(x);
                                    },
                                    x => resps.push(x)
                                }
                            };
                            let _ = state.notif.send(AtResponsePacket {
                                responses: resps,
                                status: status.unwrap()
                            });
                        }
                        else {
                            trace!("new responses: {:?}", r);
                            self.cur.as_mut().unwrap().responses.extend(r);
                        }
                    }
                    else {
                        for resp in r {
                            self.urc.unbounded_send(resp)?;
                        }
                    }
                },
                Async::NotReady => {
                    trace!("inner not ready");
                    break;
                }
            }
        }
        while let Async::Ready(r) = self.rx.poll().unwrap() {
            if let Some(r) = r {
                debug!("got a new request: {:?}", r.command);
                self.requests.push(r);
            }
            else {
                debug!("receiver ran out, future exiting");
                return Ok(Async::Ready(()));
            }
        }
        if self.cur.is_none() && self.requests.len() > 0 {
            let req = self.requests.remove(0);
            debug!("starting new request: {:?}", req.command);
            self.inner.start_send(req.command)?;
            self.cur = Some(ModemRequestState {
                notif: req.notif,
                expected: req.expected,
                responses: vec![]
            });
        }
        self.inner.poll_complete()?;
        Ok(Async::NotReady)
    }
}

