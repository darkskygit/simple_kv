use super::*;
use acid_store::{
    repo::{Compression, Encryption, LockStrategy, ObjectRepository, OpenRepo, RepositoryConfig},
    store::{DataStore, OpenOption, OpenStore, SqliteStore},
    uuid::Uuid,
};
use std::marker::PhantomData;
use std::str::FromStr;

pub use acid_store::Error as AcidError;

struct SyncAcidStore<D: DataStore>(D);

unsafe impl<D: DataStore> Sync for SyncAcidStore<D> {}

impl<D: DataStore> DataStore for SyncAcidStore<D> {
    type Error = D::Error;

    fn write_block(&mut self, id: Uuid, data: &[u8]) -> Result<(), Self::Error> {
        self.0.write_block(id, data)
    }

    fn read_block(&mut self, id: Uuid) -> Result<Option<Vec<u8>>, Self::Error> {
        self.0.read_block(id)
    }

    fn remove_block(&mut self, id: Uuid) -> Result<(), Self::Error> {
        self.0.remove_block(id)
    }

    fn list_blocks(&mut self) -> Result<Vec<Uuid>, Self::Error> {
        self.0.list_blocks()
    }
}

type AcidDb<D> = ObjectRepository<Vec<u8>, SyncAcidStore<D>>;
type AcidSqliteDb = AcidDb<SqliteStore>;
type AcidSyncDb = Arc<RwLock<AcidSqliteDb>>;

#[derive(Clone)]
pub struct AcidKVBucket<K> {
    db: AcidSyncDb,
    scope: String,
    _phantom: PhantomData<K>,
}

impl<K> AcidKVBucket<K> {
    fn new<S: ToString>(db: Arc<RwLock<AcidSqliteDb>>, scope: S) -> Self {
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
        let db = self.db.write().unwrap();
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
    db: AcidSyncDb,
}

impl AcidKV {
    fn get_store<S: ToString>(name: &S) -> Result<SyncAcidStore<SqliteStore>, AcidError> {
        use std::env::current_dir;
        Ok(SyncAcidStore(SqliteStore::open(
            current_dir()?.join(name.to_string()),
            OpenOption::CREATE,
        )?))
    }

    fn get_config() -> RepositoryConfig {
        let mut config = RepositoryConfig::default();
        config.compression = Compression::Lzma { level: 9 };
        config.encryption = Encryption::XChaCha20Poly1305;
        config
    }

    pub fn new<N: ToString>(name: N, pass: &[u8]) -> Result<Self, AcidError> {
        Ok(ObjectRepository::create_repo(
            Self::get_store(&name)?,
            Self::get_config(),
            LockStrategy::Abort,
            Some(pass),
        )
        .map(|repo| Self {
            db: Arc::new(RwLock::new(repo)),
        })
        .or_else(|e| match e {
            AcidError::AlreadyExists => ObjectRepository::open_repo(
                Self::get_store(&name)?,
                LockStrategy::Abort,
                Some(pass),
            )
            .map(|repo| Self {
                db: Arc::new(RwLock::new(repo)),
            }),
            e => Err(e),
        })?)
    }
}

impl<S: ToString> KV<S, Vec<u8>, AcidError, AcidKVBucket<S>> for AcidKV {
    fn get_bucket(&self, name: S) -> Result<AcidKVBucket<S>, AcidError> {
        Ok(AcidKVBucket::new(self.db.clone(), name))
    }
}

#[test]
#[cfg(all(feature = "zbox_kv", feature = "acid_kv"))]
pub fn transform_zbox_to_acid() -> Result<(), anyhow::Error> {
    use lazy_static::*;
    use std::time::Instant;
    lazy_static! {
        static ref DBNAME: &'static str = "old.db";
        static ref DBPASS: &'static str = "test";
    }
    ::zbox::init_env();
    let new = ZboxKV::new(*DBNAME, *DBPASS).get_bucket("")?;
    let old = AcidKV::new("acid.db", "yw4CyoTt#M8dnen@".as_bytes())?.get_bucket("")?;
    let sw = Instant::now();
    for item in old.list()? {
        let decode_sw = Instant::now();
        if let Some(data) = old.get(&get_path_string(&item)) {
            let decode_ms = decode_sw.elapsed().as_millis();
            let encode_sw = Instant::now();
            new.insert(&get_path_string(&item), data)?;
            println!(
                "move: {}, decode: {}ms, encode: {}ms",
                item.display(),
                decode_ms,
                encode_sw.elapsed().as_millis()
            );
        } else {
            println!(
                "not exist: {}, {}ms",
                item.display(),
                decode_sw.elapsed().as_millis()
            );
        }
    }
    println!("finash, {}ms", sw.elapsed().as_millis());
    Ok(())
}
