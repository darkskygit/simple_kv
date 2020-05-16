use super::*;
use sled::{Config, Db, Error as SledError};
use std::marker::PhantomData;
use std::str::FromStr;

pub struct SledKVBucket<K> {
    db: Arc<RwLock<Db>>,
    scope: String,
    _phantom: PhantomData<K>,
}

impl<K> SledKVBucket<K> {
    pub fn new<S: ToString>(db: Arc<RwLock<Db>>, scope: S) -> Self {
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

impl<K: ToString> KVBucket<K, Vec<u8>, SledError> for SledKVBucket<K> {
    fn exists(&self, k: K) -> Result<bool, SledError> {
        let db = self.db.read().unwrap();
        let path = self.get_path(k);
        Ok(db.contains_key(&path)?)
    }
    fn get(&self, k: K) -> Option<Vec<u8>> {
        let db = self.db.read().unwrap();
        let path = self.get_path(k);
        if db.contains_key(&path).unwrap_or(false) {
            db.get(&path)
                .ok()
                .and_then(|iter| iter)
                .map(|data| data.to_vec())
        } else {
            None
        }
    }
    fn insert(&self, k: K, v: Vec<u8>) -> Result<(), SledError> {
        let db = self.db.read().unwrap();
        db.insert(self.get_path(k), v)?;
        Ok(())
    }
    fn remove(&self, k: K) -> Result<(), SledError> {
        let db = self.db.read().unwrap();
        let path = self.get_path(k);
        if db.contains_key(&path)? {
            db.remove(&path)?;
        }
        Ok(())
    }
    fn list(&self) -> Result<Vec<PathBuf>, SledError> {
        let db = self.db.read().unwrap();
        let prefix = PathBuf::from(String::from_utf8_lossy(&self.get_path("")).into_owned());
        Ok(db
            .iter()
            .filter_map(|item| {
                item.ok()
                    .and_then(|(_, entry)| String::from_utf8(entry.to_vec()).ok())
                    .and_then(|path| PathBuf::from_str(&path).ok())
                    .and_then(|path| path.strip_prefix(&prefix).map(|path| path.into()).ok())
            })
            .collect())
    }
}

pub struct SledKV {
    db: Arc<RwLock<Db>>,
}

impl SledKV {
    pub fn new<N: ToString>(name: N) -> Self {
        Self {
            db: Arc::new(RwLock::new(
                Config::new()
                    .path(name.to_string())
                    .open()
                    .expect("Fail to init database"),
            )),
        }
    }
}

impl<S: ToString> KV<S, Vec<u8>, SledError, SledKVBucket<S>> for SledKV {
    fn get_bucket(&self, name: S) -> Result<SledKVBucket<S>, SledError> {
        Ok(SledKVBucket::new(self.db.clone(), name))
    }
}

#[test]
#[cfg(all(feature = "zbox_kv", feature = "sled_kv"))]
fn transform_zbox_to_sled() -> Result<(), anyhow::Error> {
    use lazy_static::*;
    use std::time::Instant;
    lazy_static! {
        static ref DBNAME: &'static str = "old.db";
        static ref DBPASS: &'static str = "test";
    }
    ::zbox::init_env();
    let old = ZboxKV::new(*DBNAME, *DBPASS).get_bucket("")?;
    let new = SledKV::new("new").get_bucket("")?;
    let sw = Instant::now();
    for item in old.list()? {
        let file_sw = Instant::now();
        if let Some(data) = old.get(&get_path_string(&item)) {
            new.insert(&get_path_string(&item), data)?;
            println!(
                "move: {}, {}ms",
                item.display(),
                file_sw.elapsed().as_millis()
            );
        } else {
            println!(
                "not exist: {}, {}ms",
                item.display(),
                file_sw.elapsed().as_millis()
            );
        }
    }
    println!("finash, {}ms", sw.elapsed().as_millis());
    Ok(())
}
