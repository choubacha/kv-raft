use super::proto;
use protobuf::{parse_from_bytes, Message};
use raft::{self, prelude::*, storage::MemStorage};
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// The file store is how we persist the state to the file system.
///
/// Generally we don't want to "confirm" that we've saved until the file
/// system comes back as ok, which in async is tough.
pub struct KeyValueCore {
    data: HashMap<String, String>,
    peers: Vec<proto::Peer>,
    file: PathBuf,
    mem: MemStorage,
}

impl KeyValueCore {
    fn new(file: PathBuf) -> Self {
        let mut core = KeyValueCore {
            data: HashMap::new(),
            mem: MemStorage::new(),
            file,
            peers: Vec::new(),
        };
        if core.file.is_file() {
            let mut handle = File::open(&core.file).unwrap();
            let mut buf = vec![];

            use std::io::Read;
            handle.read_to_end(&mut buf).unwrap();

            // This could OOM the device if the raft gets too large
            let snap = parse_from_bytes::<Snapshot>(&buf).expect("Db corrupt");
            core.apply_snapshot(snap).unwrap();
        }
        core
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.data.get(&key.to_string()).map(String::to_owned)
    }

    pub fn scan(&self) -> Vec<String> {
        self.data.keys().map(String::to_owned).collect()
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), value.to_string());
    }

    pub fn delete(&mut self, key: &str) -> Option<String> {
        self.data.remove(key)
    }

    pub fn apply_snapshot(&mut self, snapshot: Snapshot) -> raft::Result<()> {
        let snap =
            parse_from_bytes::<proto::Snap>(snapshot.get_data()).expect("Unexpected marshall err");

        let mut data = HashMap::with_capacity(snap.get_data().len());
        for datum in snap.get_data() {
            let key = datum.get_key().to_string();
            let value = datum.get_value().to_string();
            data.insert(key, value);
        }
        self.peers = snap.get_peers().iter().map(|p| p.clone()).collect();
        self.data = data;
        self.mem.wl().apply_snapshot(snapshot)
    }

    pub fn append(&mut self, ents: &[Entry]) -> raft::Result<()> {
        self.mem.wl().append(ents)
    }

    pub fn set_hardstate(&mut self, hs: HardState) {
        self.mem.wl().set_hardstate(hs);
    }

    pub fn add_node(&mut self, peer: proto::Peer) {
        if peer.get_id() == 0 || self.peers.iter().any(|p| &peer == p) {
            // Already added
            return;
        }

        self.peers.push(peer);
    }

    pub fn remove_node(&mut self, id: u64) {
        if let Some(index) = self.peers.iter().position(|p| p.id == id) {
            self.peers.remove(index);
        }
    }

    pub fn peers(&self) -> &[proto::Peer] {
        &self.peers[..]
    }

    pub fn create_snapshot<'a>(&'a mut self, idx: u64, cs: Option<ConfState>) {
        let data = self.to_snap()
            .write_to_bytes()
            .expect("Unexpected marshal err");

        if let Ok(snap) = self.mem.wl().create_snapshot(idx, cs, data) {
            use std::io::Write;

            let bytes = snap.write_to_bytes().unwrap();
            let mut file = File::create(&self.file).unwrap();
            file.write_all(&bytes).unwrap();
            println!("Snap written to disk");
        } else {
            println!("Snap not created");
        }
    }

    pub fn compact(&mut self, idx: u64) -> raft::Result<()> {
        self.mem.wl().compact(idx)
    }

    fn to_snap(&self) -> proto::Snap {
        let mut snap = proto::Snap::new();
        let mut data = Vec::with_capacity(self.data.len());
        for (k, v) in &self.data {
            let mut datum = proto::Datum::new();
            datum.set_key(k.to_owned());
            datum.set_value(v.to_owned());
            data.push(datum)
        }
        snap.set_data(data.into());
        snap.set_peers(self.peers.clone().into());
        snap
    }
}

#[derive(Clone)]
pub struct KeyValue {
    core: Arc<RwLock<KeyValueCore>>,
}

impl KeyValue {
    pub fn new<P: Into<PathBuf>>(file: P) -> Self {
        KeyValue {
            core: Arc::new(RwLock::new(KeyValueCore::new(file.into()))),
        }
    }

    pub fn wl(&self) -> RwLockWriteGuard<KeyValueCore> {
        self.core.write().unwrap()
    }

    pub fn rl(&self) -> RwLockReadGuard<KeyValueCore> {
        self.core.read().unwrap()
    }
}

impl Storage for KeyValue {
    fn initial_state(&self) -> raft::Result<RaftState> {
        self.rl().mem.initial_state()
    }

    fn entries(&self, low: u64, high: u64, max_size: u64) -> raft::Result<Vec<Entry>> {
        self.rl().mem.entries(low, high, max_size)
    }

    fn term(&self, idx: u64) -> raft::Result<u64> {
        self.rl().mem.term(idx)
    }

    fn first_index(&self) -> raft::Result<u64> {
        self.rl().mem.first_index()
    }

    fn last_index(&self) -> raft::Result<u64> {
        self.rl().mem.last_index()
    }

    fn snapshot(&self) -> raft::Result<Snapshot> {
        self.rl().mem.snapshot()
    }
}
