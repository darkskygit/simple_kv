use super::*;
use std::marker::PhantomData;
use zbox::RepoOpener;
pub use zbox::{Error as ZboxError, Repo};

#[derive(Clone)]
pub struct ZboxKVBucket<K> {
    db: Arc<RwLock<Repo>>,
    scope: PathBuf,
    _phantom: PhantomData<K>,
}

impl<K> ZboxKVBucket<K> {
    pub fn new<S: ToString>(db: Arc<RwLock<Repo>>, scope: S) -> Result<Self, ZboxError> {
        let scope = Self::create_scope(db.clone(), scope.to_string())?;
        Ok(Self {
            db,
            scope,
            _phantom: PhantomData,
        })
    }
    fn get_path<S: ToString>(&self, prefix: S) -> PathBuf {
        self.scope.join(prefix.to_string())
    }
    fn create_scope(db: Arc<RwLock<Repo>>, scope: String) -> Result<PathBuf, ZboxError> {
        let mut db = db.write().unwrap();
        let scope = PathBuf::from(if !scope.is_empty() {
            format!("/{}/", scope)
        } else {
            "/".into()
        });
        if !db.is_dir(&scope)? {
            if db.is_file(&scope)? {
                db.remove_file(&scope)?
            }
            db.create_dir(&scope)?;
        }
        Ok(scope)
    }
}

impl<K: ToString> KVBucket<K, Vec<u8>, ZboxError> for ZboxKVBucket<K> {
    fn exists(&self, k: K) -> Result<bool, ZboxError> {
        let db = self.db.read().unwrap();
        let path = self.get_path(k);
        Ok(db.is_file(&path)?)
    }
    fn get(&self, k: K) -> Option<Vec<u8>> {
        let mut db = self.db.write().unwrap();
        let path = self.get_path(k);
        if db.is_file(&path).unwrap_or(false) {
            db.open_file(&path)
                .and_then(|mut file| {
                    let mut buf = vec![];
                    file.read_to_end(&mut buf)?;
                    Ok(buf)
                })
                .ok()
        } else {
            None
        }
    }
    fn insert(&self, k: K, v: Vec<u8>) -> Result<(), ZboxError> {
        let mut db = self.db.write().unwrap();
        let path = self.get_path(k);
        if db.is_file(&path)? {
            db.remove_file(&path)?;
        }
        db.create_file(&path)?.write_once(&v)?;
        Ok(())
    }
    fn remove(&self, k: K) -> Result<(), ZboxError> {
        let mut db = self.db.write().unwrap();
        let path = self.get_path(k);
        if db.is_file(&path)? {
            db.remove_file(&path)?;
        }
        Ok(())
    }
    fn list(&self) -> Result<Vec<PathBuf>, ZboxError> {
        let db = self.db.read().unwrap();
        let prefix = self.get_path("");
        Ok(db
            .read_dir(&prefix)?
            .iter()
            .filter(|entry| entry.metadata().is_file())
            .filter_map(|entry| {
                entry
                    .path()
                    .strip_prefix(&prefix)
                    .ok()
                    .map(|path| path.into())
            })
            .collect())
    }
}

pub struct ZboxKV {
    db: Arc<RwLock<Repo>>,
}

impl ZboxKV {
    pub fn new<N: ToString, P: ToString>(name: N, pass: P) -> Self {
        Self {
            db: Arc::new(RwLock::new(
                RepoOpener::new()
                    .create(true)
                    .compress(true)
                    .dedup_chunk(true)
                    .force(true)
                    .open(&format!("sqlite://{}", name.to_string()), &pass.to_string())
                    .expect("Fail to init database"),
            )),
        }
    }
}

impl<S: ToString> KV<S, Vec<u8>, ZboxError, ZboxKVBucket<S>> for ZboxKV {
    fn get_bucket(&self, name: S) -> Result<ZboxKVBucket<S>, ZboxError> {
        ZboxKVBucket::new(self.db.clone(), name)
    }
}

#[test]
#[cfg(feature = "zbox_kv")]
fn transform_zbox() -> Result<(), anyhow::Error> {
    use lazy_static::*;
    use std::time::Instant;
    lazy_static! {
        static ref DBNAME: &'static str = "old.db";
        static ref DBPASS: &'static str = "test";
    }
    ::zbox::init_env();
    let old = ZboxKV::new(*DBNAME, *DBPASS).get_bucket("")?;
    let new = ZboxKV::new("new.db", "test").get_bucket("")?;
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
