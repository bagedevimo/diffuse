extern crate hex;

use std::io;

fn main() {
    let input = io::stdin();

    print!("0000000000000000000000000000000000000000 capabilities^{{}}");
    print!("\0");
    print!("report-status delete-refs side-band-64k quiet atomic ofs-delta push-options agent=git/2.21.0");
    print!("\n");

    let mut conn = diffuse::git::Connection::new(Box::new(input));

    loop {
        let packet = match conn.receive_packet() {
            Ok(p) => p,
            Err(diffuse::git::ConnectionResult::EndOfStream) => break,
            Err(e) => panic!("Error receiving packet: {:?}", e),
        };

        match packet {
            diffuse::git::Packet::Message { size, data } => {
                eprintln!("Packet ({:?}): {:?}", size, String::from_utf8_lossy(&data))
            }
            diffuse::git::Packet::Pack => eprintln!("Pack."),
        }
    }

    eprintln!("End of stream");
}
