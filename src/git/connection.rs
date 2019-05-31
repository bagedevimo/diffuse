use crate::util;
use std::io::Read;

#[derive(Debug)]
pub enum Packet {
    Message { size: usize, data: Vec<u8> },
    Pack,
}

const PACK_HEADER: [u8; 4] = [b'P', b'A', b'C', b'K'];
const NIL_HEADER: [u8; 4] = [0; 4];

pub struct Connection {
    stream: Box<dyn std::io::Read>,
}

impl Connection {
    pub fn new(str: Box<dyn std::io::Read>) -> Connection {
        Connection { stream: str }
    }

    pub fn receive_packet(&mut self) -> Result<Packet, ConnectionResult> {
        let mut header: [u8; 4] = [0; 4];
        self.stream.read(&mut header)?;

        eprintln!("Header: {:?}", header);

        match header {
            PACK_HEADER => self.receive_pack(),
            NIL_HEADER => Err(ConnectionResult::EndOfStream),
            _ => self.receive_message(header),
        }
    }

    fn receive_pack(&mut self) -> Result<Packet, ConnectionResult> {
        parse_pack(&mut self.stream);
        Ok(Packet::Pack)
    }

    fn receive_message(&mut self, header: [u8; 4]) -> Result<Packet, ConnectionResult> {
        let header_bytes = util::ascii_hex_to_bytes(&header.to_vec());
        let mut header_bytes_array = [0; 4];
        header_bytes_array.copy_from_slice(&header_bytes[..4]);

        let size = util::as_u32_be(&header_bytes_array) as usize;

        if size == 0 {
            return Err(ConnectionResult::EndOfStream);
        }

        let mut buffer = Vec::new();
        let mut read_buffer = self.stream.as_mut().take((size) as u64);
        read_buffer.read_to_end(&mut buffer);

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

fn parse_pack<T: Read>(reader: &mut T) {
    let mut version_bytes: [u8; 4] = [0; 4];
    reader
        .read(&mut version_bytes)
        .expect("unexpected EOF while reading pack version");

    let mut pack_object_count_bytes: [u8; 4] = [0; 4];
    reader
        .read(&mut pack_object_count_bytes)
        .expect("unexpected EOF while reading pack object count");

    let pack_object_count = util::as_u32_be(&pack_object_count_bytes);

    let mut pack_objects: Vec<PackObject> = Vec::new();
    for pack_index in 0..pack_object_count {
        let pack_object = parse_pack_object_record(reader);
        pack_objects.push(pack_object);
    }

    let mut trailer_signature: [u8; 20] = [0; 20];
    reader
        .read(&mut trailer_signature)
        .expect("unexpected EOF while reading pack signature");

    let commit_count: usize = pack_objects.iter().filter(|x| x.object_type == 1).count();
    let tree_count: usize = pack_objects.iter().filter(|x| x.object_type == 2).count();
    let blob_count: usize = pack_objects.iter().filter(|x| x.object_type == 3).count();
    eprintln!(
        "Pack contains {} commits, {} trees and {} blobs",
        commit_count, tree_count, blob_count
    );
}

// def read_record_header
//   byte, size = Numbers::VarIntLE.read(@input, 4)
//   type = (byte >> 4) & 0x7

//   [type, size]
// end

// def self.read(input, shift)
//   first = input.readbyte
//   value = first & (2 ** shift - 1)

//   byte = first

//   until byte < 0x80
//     byte   = input.readbyte
//     value |= (byte & 0x7f) << shift
//     shift += 7
//   end

//   [first, value]
// end

#[cfg(test)]
mod tests {
    #[test]
    fn read_variable_length_int_test() {
        let mut raw_data: [u8; 14] = [157, 11, 120, 156, 165, 204, 49, 14, 66, 33, 12, 0, 208, 15];
        let mut raw_data_slice = &raw_data[..];

        let (byte, value) = crate::git::connection::read_variable_length_int(&mut raw_data_slice);
        assert_eq!(byte, 157);
        assert_eq!(value, 189);
    }
}

fn read_byte<T: Read>(reader: &mut T) -> Option<u8> {
    let mut byte: [u8; 1] = [0; 1];

    match reader.read(&mut byte) {
        Ok(_) => Some(*byte.first().unwrap()),
        Err(e) => panic!(e),
    }
}

fn read_variable_length_int<T: Read>(reader: &mut T) -> (u8, u32) {
    let mut shift = 4;

    // let first: u8 = 0x00;
    let first: u8 = read_byte(reader).unwrap();
    // input.read(&first).expect();

    let mut value: u32 = (first as u32) & (u32::pow(2, shift) - 1);

    let mut byte = first;

    while byte >= 0x80 {
        byte = read_byte(reader).unwrap();
        // eprintln!("Byte: {:?}, shift: {:?}, value: {:?}", byte, shift, value);
        value |= ((byte & 0x7F) as u32) << shift;
        shift += 7;
    }

    (first, value)
}

struct PackObject {
    object_type: u8,
    data: Vec<u8>,
}

fn parse_pack_object_record<T: Read>(reader: &mut T) -> PackObject {
    let (byte, size) = read_variable_length_int(reader);
    let record_type = (byte >> 4) & 0x7;

    // eprintln!("record_type: {:?}, size: {:?}", record_type, size);

    let data = inflate_record_data(reader, size as u64);

    PackObject {
        object_type: record_type,
        data: data,
    }
}

fn inflate_record_data<T: Read>(reader: &mut T, expected_bytes: u64) -> Vec<u8> {
    // eprintln!("Trying to deflate record");
    // eprintln!(
    //     "Peek: {:?} {:?}",
    //     read_byte(reader).unwrap(),
    //     read_byte(reader).unwrap()
    // );

    // reader.seek(SeekFrom::Current(-2));
    let mut deflater = flate2::Decompress::new(true);

    let mut output: Vec<u8> = Vec::with_capacity(expected_bytes as usize);
    // let mut data = Vec::new();

    loop {
        let mut in_byte: [u8; 1] = [0; 1];
        reader.read(&mut in_byte);

        let mut out_bytes: [u8; 16] = [0; 16];

        let status = deflater.decompress_vec(&in_byte, &mut output, flate2::FlushDecompress::None);

        // if deflater.total_out() > output.len() as u64 {
        //     for i in 0..(deflater.total_out() - output.len() as u64) {
        //         output.push(out_bytes[i as usize]);
        //     }
        //     // output.push(*out_byte.first().unwrap());
        // }

        // eprintln!(
        //     "in: {:?}, out: {:?}, status: {:?}, total_out: {:?}, expecting: {:?}, \noutput:\n{}\n\n",
        //     in_byte,
        //     out_bytes,
        //     status,
        //     deflater.total_out(),
        //     expected_bytes,
        //     String::from_utf8_lossy(&output.clone()),
        // );

        match status {
            Ok(inner_status) => {
                // eprintln!("inner_status: {:?}", inner_status);
                match inner_status {
                    flate2::Status::StreamEnd => {
                        // eprintln!("Stream end?");
                        break;
                    }
                    flate2::Status::Ok => {}
                    flate2::Status::BufError => panic!("Here"),
                };
            }
            Err(e) => panic!("Error: {:?}, {:?}", e, e.needs_dictionary()),
        }

        // if deflater.total_out() == expected_bytes {
        //     break;
        // }
    }

    output

    // match maybe_byte {
    //     Some(byte) => {
    //         println!("Decompressor: {:?}", byte);
    //         data.push(byte)
    //     }
    //     None => break,
    // };
    // }

    // deflater.read_to_string(&mut data).unwrap();

    // eprintln!("Decompression:\n{:?}\n\n", data);
}
