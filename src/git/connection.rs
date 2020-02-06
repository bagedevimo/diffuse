use crate::util;
use std::io::Read;
use std::io::Seek;

use crate::git::database::Database;
use crate::git::database::Record;

pub enum Packet {
    Message { size: usize, data: Vec<u8> },
    Pack { records: Vec<Record> },
}

const PACK_HEADER: [u8; 4] = [b'P', b'A', b'C', b'K'];
const NIL_HEADER: [u8; 4] = [0, 0, 0, 0];

const RECORD_TYPE_COMMIT: u8 = 1;
const RECORD_TYPE_TREE: u8 = 2;
const RECORD_TYPE_BLOB: u8 = 3;

const RECORD_TYPE_OFS_DELTA: u8 = 6;
const RECORD_TYPE_REF_DELTA: u8 = 7;

pub const GIT_MAX_COPY: u64 = 0x10000;

pub struct Connection<'a> {
    stream: std::io::Cursor<Vec<u8>>,
    database: &'a mut Database,
}

impl<'a> Connection<'a> {
    pub fn new(str: std::io::Cursor<Vec<u8>>, database: &mut Database) -> Connection {
        Connection {
            stream: str,
            database: database,
        }
    }

    pub fn receive_packet(&mut self) -> Result<Packet, ConnectionResult> {
        let mut header: [u8; 4] = [0; 4];
        self.stream.read(&mut header)?;

        match header {
            self::PACK_HEADER => self.receive_pack(),
            self::NIL_HEADER => Err(ConnectionResult::EndOfStream),
            _ => self.receive_message(header),
        }
    }

    pub fn get_database(&self) -> &Database {
        self.database
    }

    fn receive_pack(&mut self) -> Result<Packet, ConnectionResult> {
        parse_pack(&mut self.stream, self.database);
        Ok(Packet::Pack {
            records: Vec::new(),
        })
    }

    fn receive_message(&mut self, header: [u8; 4]) -> Result<Packet, ConnectionResult> {
        let header_bytes = util::ascii_hex_to_bytes(&header.to_vec());
        let mut header_bytes_array = [0; 4];
        header_bytes_array.copy_from_slice(&header_bytes[..4]);

        let size = util::as_u32_be(&header_bytes_array) as usize;

        let mut buffer = Vec::new();

        if size > 0 {
            let mut read_buffer = self.stream.clone().take((size - 4) as u64);
            self.stream
                .seek(std::io::SeekFrom::Current(size as i64 - 4))
                .unwrap();
            match read_buffer.read_to_end(&mut buffer) {
                Ok(_) => {}
                Err(e) => panic!("Unexpected EOF while reading message: {}", e),
            }
        }

        Ok(Packet::Message {
            size: size,
            data: buffer,
        })
    }
}

#[derive(Debug)]
pub enum ConnectionResult {
    IOError(std::io::Error),
    EndOfStream,
}

impl std::convert::From<std::io::Error> for ConnectionResult {
    fn from(error: std::io::Error) -> Self {
        ConnectionResult::IOError(error)
    }
}

// CLEAN UP

fn parse_pack<T: Read>(reader: &mut T, database: &mut Database) -> Vec<Record> {
    let mut version_bytes: [u8; 4] = [0; 4];
    reader
        .read(&mut version_bytes)
        .expect("unexpected EOF while reading pack version");

    let mut pack_object_count_bytes: [u8; 4] = [0; 4];
    reader
        .read(&mut pack_object_count_bytes)
        .expect("unexpected EOF while reading pack object count");

    let pack_object_count = util::as_u32_be(&pack_object_count_bytes);

    let mut pack_objects: Vec<crate::git::database::Record> = Vec::new();
    for _pack_index in 0..pack_object_count {
        let pack_object = parse_pack_object_record(reader, database);
        pack_objects.push(pack_object.clone());

        eprintln!("Inserting..");
        database.insert(pack_object);
        eprintln!("Inserting done");
    }

    let mut trailer_signature: [u8; 20] = [0; 20];
    reader
        .read(&mut trailer_signature)
        .expect("unexpected EOF while reading pack signature");

    // let commit_count: usize = pack_objects
    //     .iter()
    //     .filter(|x| match x {
    //         crate::git::database::Record::Commit { .. } => true,
    //         _ => false,
    //     })
    //     .count();

    // let tree_count: usize = pack_objects
    //     .iter()
    //     .filter(|x| match x {
    //         crate::git::database::Record::Tree { .. } => true,
    //         _ => false,
    //     })
    //     .count();
    // let blob_count: usize = pack_objects
    //     .into_iter()
    //     .filter(|x| match x {
    //         crate::git::database::Record::Blob { .. } => true,
    //         _ => false,
    //     })
    //     .count();

    pack_objects
}

#[cfg(test)]
mod tests {
    #[test]
    fn read_variable_length_int_test() {
        let mut raw_data: [u8; 2] = [210, 35];
        let mut raw_data_slice = &raw_data[..];

        let (byte, value) =
            crate::git::connection::read_variable_length_int(&mut raw_data_slice, 7);
        assert_eq!(byte, 210);
        assert_eq!(value, 4562);
    }
}

fn read_byte(reader: &mut impl Read) -> Option<u8> {
    let mut byte: [u8; 1] = [0; 1];

    match reader.read(&mut byte) {
        Ok(_) => Some(*byte.first().unwrap()),
        Err(e) => panic!(e),
    }
}

pub fn read_variable_length_int(reader: &mut impl Read, shift_start: u32) -> (u8, u32) {
    let mut shift = shift_start;

    let first: u8 = read_byte(reader).unwrap();

    let mut value: u32 = (first as u32) & (u32::pow(2, shift) - 1);

    let mut byte = first;

    while byte >= 0x80 {
        byte = read_byte(reader).unwrap();

        value |= ((byte & 0x7F) as u32) << shift;
        shift += 7;
    }

    (first, value)
}

fn parse_pack_object_record(
    reader: &mut impl Read,
    database: &mut Database,
) -> crate::git::database::Record {
    let (byte, _) = read_variable_length_int(reader, 4);
    let record_type = (byte >> 4) & 0x7;

    let record: crate::git::database::Record = match record_type {
        RECORD_TYPE_COMMIT => crate::git::record::parse_commit(inflate_record_data(reader).0),
        RECORD_TYPE_TREE => crate::git::record::parse_tree(inflate_record_data(reader).0),
        RECORD_TYPE_BLOB => crate::git::record::parse_blob(inflate_record_data(reader).0),
        RECORD_TYPE_OFS_DELTA => {
            // let (byte, value) = crate::git::connection::read_variable_length_int(reader);
            // eprintln!("byte: {:?}, value: {:?}", byte, value);
            // let (byte, value) = crate::git::connection::read_variable_length_int(reader);
            // eprintln!("byte: {:?}, value: {:?}", byte, value);
            crate::git::record::parse_ofs_delta(inflate_xdelta_record(reader, database))
        }
        RECORD_TYPE_REF_DELTA => {
            crate::git::record::parse_ref_delta(inflate_xdelta_record(reader, database))
        }
        // 6...7 => Record::Unknown {
        //     data: inflate_xdelta_record(reader, size as u64),
        // },
        x => panic!("Unknown record type: {}", x),
    };

    // if record_type == RECORD_TYPE_TREE {
    // eprintln!(
    //     "Record ({}):\n{:?}\n\n",
    //     // record.get_name(),
    //     get_object_id(&record),
    //     record,
    // );
    // }

    record
}

fn inflate_xdelta_record<T: Read>(mut reader: &mut T, database: &mut Database) -> Vec<u8> {
    let mut oid_bytes = [0; 20];
    reader.read(&mut oid_bytes).unwrap();

    let source_id = crate::git::database::ObjectID::from_oid_bytes(oid_bytes);
    let source_object = database.fetch(&source_id);

    // let (byte, value) = crate::git::connection::read_variable_length_int(reader);
    // eprintln!("byte: {:?}, value: {:?}", byte, value as i8);

    // let (byte, value) = crate::git::connection::read_variable_length_int(reader);
    // eprintln!("byte: {:?}, value: {:?}", byte, value as i8);

    let (bytes, _) = inflate_record_data(&mut reader);
    let mut secondary_cursor = std::io::Cursor::new(bytes.clone());

    let (_, _v1) = crate::git::connection::read_variable_length_int(&mut secondary_cursor, 7);
    let (_, _v2) = crate::git::connection::read_variable_length_int(&mut secondary_cursor, 7);

    let mut out_buffer = Vec::new();

    while secondary_cursor.stream_position().unwrap() < secondary_cursor.stream_len().unwrap() {
        let mut peek: [u8; 1] = [0; 1];
        secondary_cursor.read(&mut peek).unwrap();

        // secondary_cursor.seek(std::io::SeekFrom::Current(-1));

        if peek[0] < 0x80 {
            // let (_, size) = crate::git::connection::read_variable_length_int(&mut reader, 4);

            // let mut buffer = vec![0u8; size as usize];
            // reader.read_exact(&mut buffer).unwrap();
            for _ in 0..peek[0] {
                let mut new_byte: [u8; 1] = [0; 1];
                secondary_cursor.read(&mut new_byte);
                out_buffer.push(new_byte[0]);
            }
        } else {
            let value =
                crate::git::record::read_packed_int_56le(&mut secondary_cursor, peek[0] as u64);
            let offset = value & 0xffffffff;
            let size = value >> 32;

            let actual_size = if size == 0 {
                crate::git::connection::GIT_MAX_COPY
            } else {
                size
            };

            match source_object {
                Some(so) => {
                    let data = match so {
                        Record::Commit { data, .. } => data,
                        Record::Tree { data, .. } => data,
                        Record::Blob { data, .. } => data,
                    };

                    let mut bytes_to_copy: Vec<u8> = vec![0; actual_size as usize];

                    bytes_to_copy
                        .copy_from_slice(&data[offset as usize..(offset + actual_size) as usize]);
                    // let bytes_to_copy = data[offset as usize..size as usize];

                    // eprintln!("Copying\n{}\n\n", String::from_utf8_lossy(&bytes_to_copy));
                    out_buffer.append(&mut bytes_to_copy);
                }
                None => {
                    eprintln!(
                        "WARNING: Forced to skip XDELTA decompression because we can't find {}",
                        source_id
                    );
                }
            };
        }
    }

    out_buffer
}

fn inflate_record_data<T: Read>(reader: &mut T) -> (Vec<u8>, u64) {
    let mut deflater = flate2::Decompress::new(true);

    let mut output: Vec<u8> = Vec::with_capacity(65000 as usize);
    let mut input: Vec<u8> = Vec::new();

    loop {
        let mut in_byte: [u8; 1] = [0; 1];

        match reader.read(&mut in_byte) {
            Ok(_) => {}
            Err(e) => panic!("Unexpected EOF inflating DEFLATE stream: {}", e),
        }

        input.push(in_byte[0]);

        let status = deflater.decompress_vec(&in_byte, &mut output, flate2::FlushDecompress::None);

        match status {
            Ok(flate2::Status::StreamEnd) => break,
            Ok(flate2::Status::Ok) => {}
            Ok(flate2::Status::BufError) => panic!("Inflate buffer error"),
            Err(e) => panic!("Error: {:?}", e),
        }
    }

    (output, deflater.total_in())
}
