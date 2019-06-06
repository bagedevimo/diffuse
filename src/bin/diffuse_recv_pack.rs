extern crate hex;

use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;

use std::io::Seek;

fn main() {
    let input = io::stdin();

    print!("0000000000000000000000000000000000000000 capabilities^{{}}");
    print!("\0");
    print!("report-status delete-refs side-band-64k quiet atomic ofs-delta push-options agent=git/2.21.0");
    print!("\n");

    let mut buffer = Vec::new();
    std::io::stdin().lock().read_to_end(&mut buffer);
    let mut cursor = std::io::Cursor::new(buffer);

    let mut content = Vec::new();
    cursor.read_to_end(&mut content);

    let mut f = File::create("foo.txt").unwrap();
    f.write_all(&content).unwrap();

    cursor.seek(std::io::SeekFrom::Start(0));

    let mut conn = diffuse::git::Connection::new(cursor);

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
        "REMOTE: Server has {} objects in database",
        conn.get_database().object_count()
    );

    eprintln!("REMOTE: Server object listing");
    conn.get_database().dump();
    eprintln!("REMOTE: End server object listing");
}
