use std::io::BufRead;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;

use crate::git::database::Record;

pub fn parse_commit(d: Vec<u8>) -> Record {
    Record::Commit {
        data: d.clone(),
        commit_info: String::from_utf8(d).unwrap(),
    }
}

pub fn parse_tree(d: Vec<u8>) -> Record {
    let mut entries = Vec::new();

    let mut reader = Cursor::new(d.clone());

    while reader.stream_position().unwrap() < reader.stream_len().unwrap() {
        let mut mode = Vec::new();
        reader.read_until(b' ', &mut mode);

        let mut name = Vec::new();
        reader.read_until(b'\0', &mut name);

        let name_str = String::from_utf8_lossy(&name).to_string();

        let mut oid = [0; 20];
        reader.read(&mut oid);

        let entry = crate::git::database::TreeEntry {
            mode: mode,
            name: name_str,
            oid: hex::encode(oid),
        };

        entries.push(entry);
    }

    Record::Tree {
        data: d.clone(),
        entries: entries,
    }
}

pub fn parse_blob(d: Vec<u8>) -> Record {
    Record::Blob { data: Vec::from(d) }
}

pub fn parse_ofs_delta(d: Vec<u8>) -> Record {
    panic!("OFSDelta is not implemented!");
    Record::Blob { data: Vec::from(d) }
}

pub fn parse_ref_delta(d: Vec<u8>) -> Record {
    Record::Blob { data: Vec::from(d) }
}

pub fn read_packed_int_56le(input: &mut impl Read, header: u64) -> u64 {
    let iter = (0 as u64..6 as u64)
        .into_iter()
        .filter(|i| header & (1 << (*i as u64)) != 0);

    let iter_collected: Vec<u64> = iter.clone().collect();

    let bytes = iter.map(|i| {
        let mut bufs = [0; 1];
        input.read(&mut bufs);

        let byte = bufs[0];

        (byte as i64).checked_shl((i as u32 * 8)).unwrap()
    });

    bytes.fold(0, |a, b| a | b as u64)
}
