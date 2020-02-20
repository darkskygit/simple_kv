use super::*;
use acid_store::{
    repo::{Compression, Encryption, ObjectRepository, RepositoryConfig},
    store::{DataStore, Open, OpenOption, SqliteStore},
    uuid::Uuid,
};
use std::marker::PhantomData;
use std::str::FromStr;

type AcidDb = ObjectRepository<Vec<u8>, SqliteStore>;

pub struct AcidKVBucket<K> {
    db: Arc<RwLock<AcidDb>>,
    scope: String,
    _phantom: PhantomData<K>,
}

impl<K> AcidKVBucket<K> {
    pub fn new<S: ToString>(db: Arc<RwLock<AcidDb>>, scope: S) -> Self {
        Self {
            db,
            scope: scope.to_string(),
            _phantom: PhantomData,
        }
    }
    fn get_path<S: ToString>(&self, prefix: S) -> Vec<u8> {
        format!(
            "{}{}",
            if !self.scope.is_empty() {
                format!("/{}/", self.scope)
            } else {
                "/".into()
            },
            prefix.to_string()
        )
        .into_bytes()
    }
}

impl<K: ToString> KVBucket<K, Vec<u8>, AcidError> for AcidKVBucket<K> {
    fn exists(&self, k: K) -> Result<bool, AcidError> {
        let db = self.db.read().unwrap();
        let path = self.get_path(k);
        Ok(db.contains(&path))
    }
    fn get(&self, k: K) -> Option<Vec<u8>> {
        let mut db = self.db.write().unwrap();
        let path = self.get_path(k);
        if db.contains(&path) {
            db.get(&path).and_then(|mut obj| {
                let mut buf = vec![];
                if obj.read_to_end(&mut buf).is_ok() {
                    Some(buf)
                } else {
                    None
                }
            })
        } else {
            None
        }
    }
    fn insert(&self, k: K, mut v: Vec<u8>) -> Result<(), AcidError> {
        let mut db = self.db.write().unwrap();
        let mut obj = db.insert(self.get_path(k));
        obj.write_all(&mut v)?;
        obj.flush()?;
        Ok(())
    }
    fn remove(&self, k: K) -> Result<(), AcidError> {
        let mut db = self.db.write().unwrap();
        let path = self.get_path(k);
        if db.contains(&path) {
            db.remove(&path);
        }
        Ok(())
    }
    fn list(&self) -> Result<Vec<PathBuf>, AcidError> {
        let db = self.db.read().unwrap();
        let prefix = PathBuf::from(String::from_utf8_lossy(&self.get_path("")).into_owned());
        Ok(db
            .keys()
            .filter_map(|item| {
                String::from_utf8(item.to_vec())
                    .ok()
                    .and_then(|path| PathBuf::from_str(&path).ok())
                    .and_then(|path| path.strip_prefix(&prefix).map(|path| path.into()).ok())
            })
            .collect())
    }
}

pub struct AcidKV {
    db: Arc<RwLock<AcidDb>>,
}

impl AcidKV {
    pub fn new<N: ToString>(name: N, pass: &[u8]) -> Result<Self, AcidError> {
        use std::env::current_dir;
        Ok(Self {
            db: Arc::new(RwLock::new(ObjectRepository::create_repo(
                SqliteStore::open(current_dir()?.join(name.to_string()), OpenOption::CREATE)?,
                RepositoryConfig {
                    chunker_bits: 20,
                    compression: Compression::Lzma { level: 9 },
                    encryption: Encryption::XChaCha20Poly1305,
                    ..Default::default()
                },
                Some(pass),
            )?)),
        })
    }
}

impl<S: ToString> KV<S, Vec<u8>, AcidError, AcidKVBucket<S>> for AcidKV {
    fn get_bucket(&self, name: S) -> Result<AcidKVBucket<S>, AcidError> {
        Ok(AcidKVBucket::new(self.db.clone(), name))
    }
}

#[test]
#[cfg(all(feature = "zbox_kv", feature = "acid_kv"))]
fn transform_zbox_to_acid() -> Result<(), exitfailure::ExitFailure> {
    use lazy_static::*;
    use stopwatch::Stopwatch;
    lazy_static! {
        static ref DBNAME: &'static str = "old.db";
        static ref DBPASS: &'static str = "test";
    }
    ::zbox::init_env();
    let old = ZboxKV::new(*DBNAME, *DBPASS).get_bucket("")?;
    let new = AcidKV::new("acid.db", DBPASS.as_bytes())?.get_bucket("")?;
    let sw = Stopwatch::start_new();
    for item in old.list()? {
        let decode_sw = Stopwatch::start_new();
        if let Some(data) = old.get(&get_path_string(&item)) {
            let decode_ms = decode_sw.elapsed_ms();
            let encode_sw = Stopwatch::start_new();
            new.insert(&get_path_string(&item), data)?;
            println!(
                "move: {}, decode: {}ms, encode: {}ms",
                item.display(),
                decode_ms,
                encode_sw.elapsed_ms()
            );
        } else {
            println!(
                "not exist: {}, {}ms",
                item.display(),
                decode_sw.elapsed_ms()
            );
        }
    }
    println!("finash, {}ms", sw.elapsed_ms());
    Ok(())
}
