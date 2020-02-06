#![feature(async_await)]

use std::io::{self, Read};

pub fn run() -> io::Result<()> {
    let input = io::stdin();

    print!("0000000000000000000000000000000000000000 capabilities^{{}}");
    print!("\0");
    print!(
        "report-status delete-refs side-band-64k quiet atomic push-options agent=diffuse/2.21.0"
    );
    print!("\n");

    let mut buffer = Vec::new();
    std::io::stdin().lock().read_to_end(&mut buffer);
    let mut cursor = std::io::Cursor::new(buffer);

    let mut database = diffuse::git::Database::new();
    let mut conn = diffuse::git::Connection::new(cursor, &mut database);

    loop {
        let packet = match conn.receive_packet() {
            Ok(p) => p,
            Err(diffuse::git::ConnectionResult::EndOfStream) => break,
            Err(e) => panic!("Error receiving packet: {:?}", e),
        };

        match packet {
            diffuse::git::Packet::Message { size, data } => {
                // eprintln!("Packet ({:?}): {:?}", size, String::from_utf8_lossy(&data))
            }
            diffuse::git::Packet::Pack { records: _ } => {}
        }
    }

    eprintln!(
        "remote:\nremote: Server has {} objects in database\nremote:",
        conn.get_database().object_count()
    );

    eprintln!("REMOTE: Server object listing");
    conn.get_database().dump();
    eprintln!("REMOTE: End server object listing");

    Ok(())
}

fn main() {
    run().unwrap()
}
