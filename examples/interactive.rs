#![feature(try_from)]

extern crate huawei_modem;
extern crate env_logger;
extern crate futures;
extern crate tokio_core;

use tokio_core::reactor::Core;
use futures::{Future, Stream};
use huawei_modem::{HuaweiModem, cmd};
use huawei_modem::pdu::{GsmMessageData, Pdu, PduAddress, HexData};
use std::convert::TryFrom;
use std::io::prelude::*;

fn main() {

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
    println!("Setting new message indications...");
    let fut = cmd::sms::set_new_message_indications(&mut modem,
                                                    cmd::sms::NewMessageNotification::SendDirectlyOrBuffer,
                                                    cmd::sms::NewMessageStorage::StoreAndNotify);
    println!("Result: {:?}", core.run(fut));
    println!("\n### Instructions for use ###");
    println!("- Read messages by typing 'read'");
    println!("- Send messages by typing '[recipient];[message]', replacing [recipient] with the phone number and [message] with the message");
    println!("- Delete all messages by typing 'del'");
    println!("");
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
                        let dec = msg.pdu.get_message_data().decode_message();
                        match dec {
                            Ok(dm) => {
                                println!("Text: {}", dm.text);
                                if let Some(u) = dm.udh {
                                    println!("User data header: {:?}", u);
                                }
                            },
                            Err(e) => {
                                println!("Decode failed: {}", e);
                            },
                        }
                    }
                });
            println!("Result: {:?}", core.run(fut));
            continue;
        }
        if ln == "del" {
            println!("Deleting messags...");
            let fut = cmd::sms::del_sms_pdu(&mut modem, cmd::sms::DeletionOptions::DeleteAll);
            println!("Result: {:?}", core.run(fut));
            continue;
        }
        let ln = ln.split(";").collect::<Vec<_>>();
        println!("Sending \"{}\" to {}...", ln[1], ln[0]);
        let recipient: PduAddress = ln[0].parse().unwrap();
        println!("Recipient: {:?}", recipient);
        let msg = GsmMessageData::encode_message(ln[1]);
        println!("Message data: {:?}", msg);
        for msg in msg {
            let msg = Pdu::make_simple_message(recipient.clone(), msg);
            println!("PDU: {:?}", msg);
            println!("Encoded PDU: {}", HexData(&msg.as_bytes().0));
            let pdu = Pdu::try_from(&msg.as_bytes().0 as &[u8]).unwrap();
            assert_eq!(pdu, msg);
            let fut = cmd::sms::send_sms_pdu(&mut modem, &msg);
            println!("Result: {:?}", core.run(fut));
        }
    }
}
