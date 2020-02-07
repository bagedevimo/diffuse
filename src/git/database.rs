use crypto::digest::Digest;
use crypto::sha1::Sha1;
use std::collections::HashMap;

pub struct Database {
    pub entries: HashMap<ObjectID, Record>,
}

impl Database {
    pub fn new() -> Database {
        Database {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, record: Record) -> Option<ObjectID> {
        let object_id_str = get_object_id(&record);
        let object_id = ObjectID::from_oid_string(object_id_str);

        self.entries.insert(object_id.clone(), record.clone());

        Some(object_id)
    }

    pub fn fetch(&self, oid: &ObjectID) -> Option<&Record> {
        self.entries.get(oid)
    }

    pub fn object_count(&self) -> usize {
        self.entries.len() as usize
    }

    pub fn dump(&self) {
        for (object_id, _) in &self.entries {
            eprintln!("{}", object_id);
        }
    }
}

#[derive(Clone)]
pub struct ObjectID {
    oid_bytes: [u8; 20],
    oid_string: String,
}

impl ObjectID {
    pub fn from_oid_bytes(bytes: [u8; 20]) -> ObjectID {
        ObjectID {
            oid_bytes: bytes,
            oid_string: hex::encode(bytes),
        }
    }

    pub fn from_oid_string(string: String) -> ObjectID {
        let oid_bytes_vec = string.as_bytes();
        let oid_bytes_slice = hex::decode(oid_bytes_vec).unwrap();
        let mut bytes: [u8; 20] = [0; 20];
        bytes.copy_from_slice(&oid_bytes_slice);

        ObjectID {
            oid_bytes: bytes,
            oid_string: string,
        }
    }
}

impl std::cmp::PartialEq for ObjectID {
    fn eq(&self, other: &Self) -> bool {
        self.oid_string == other.oid_string
    }
}

impl Eq for ObjectID {}

impl std::hash::Hash for ObjectID {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.oid_string.hash(state);
    }
}

impl std::fmt::Display for ObjectID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ObjectID({})", self.oid_string)
    }
}

#[derive(Clone, Debug)]
pub enum Record {
    Commit {
        data: Vec<u8>,
        commit_info: String,
    },
    Tree {
        data: Vec<u8>,
        entries: Vec<crate::git::database::TreeEntry>,
    },
    Blob {
        data: Vec<u8>,
    },
}

#[derive(Clone, Debug)]
pub struct TreeEntry {
    pub mode: Vec<u8>,
    pub name: String,
    pub oid: String,
}

pub fn get_object_id(record: &Record) -> String {
    let (name, data) = match record {
        Record::Commit { data, .. } => ("commit", data),
        Record::Tree { data, .. } => ("tree", data),
        Record::Blob { data, .. } => ("blob", data),
    };

    let size = data.len();

    let mut hash_data: Vec<u8> = Vec::new();

    let mut name_data = name.as_bytes().to_vec();
    hash_data.append(&mut name_data);
    hash_data.push(b' ');

    let mut size_data = format!("{}", size).as_bytes().to_vec();
    hash_data.append(&mut size_data);

    hash_data.push(b'\0');
    hash_data.append(&mut data.clone());

    let mut hasher = Sha1::new();
    hasher.input(&hash_data);

    // let mut f = std::fs::File::open("dumped_object").unwrap();
    // f.write(&hash_data);

    if size == 377 {
        std::fs::write("dumped_object", &hash_data).unwrap();
        eprintln!(
            "{} is \n{:?}\n\n",
            hasher.result_str(),
            hex::encode(hash_data)
        );
    }
    hasher.result_str()
}
