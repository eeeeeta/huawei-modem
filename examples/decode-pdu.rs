use huawei_modem::pdu::{DeliverPdu, HexData};
use std::convert::TryFrom;
use std::io::prelude::*;

fn main() {
    println!("Input PDUs");
    let stdin = ::std::io::stdin();
    let lock = stdin.lock();
    for ln in lock.lines() {
        let ln = ln.unwrap();
        let bytes = HexData::decode(&ln).unwrap();
        let pdu = DeliverPdu::try_from(&bytes as &[u8]).unwrap();
        println!("PDU: {:?}", pdu);
        let data = pdu.get_message_data().decode_message();
        println!("Sender: {}", pdu.originating_address);
        println!("Message: {:?}", data);
    }
}
