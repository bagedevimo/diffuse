use std::fs::{self, File};
use std::io::prelude::*;
use std::io::{self, BufRead};

struct Packet {
    data: Vec<u8>,
}

impl Packet {
    fn new(d: Vec<u8>) -> Packet {
        Packet { data: d }
    }
}

fn main() {
    let stdin = io::stdin();
    let mut packets: Vec<Packet> = vec![];
    let mut file = File::create("out.log").expect("cannot write file");

    loop {
        let mut buffer: Vec<u8> = vec![];

        let result = stdin
            .lock()
            .read_until(b'\0', &mut buffer)
            .expect("unexpected EOF");

        if result <= 0 {
            break;
        }

        let packet = Packet::new(buffer.clone());
        packets.push(packet);
    }

    for packet in packets {
        file.write_all(b"====================\n");
        file.write_all(&packet.data);
        file.write_all(b"====================\n\n");
    }
}
