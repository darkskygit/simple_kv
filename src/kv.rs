use std::path::PathBuf;

pub trait KV<K, V, E, B: KVBucket<K, V, E>> {
    fn get_bucket(&self, name: K) -> B;
}

pub trait KVBucket<K, V, E> {
    fn exists(&self, k: K) -> Result<bool, E>;
    fn get(&self, k: K) -> Option<V>;
    fn insert(&self, k: K, v: V) -> Result<(), E>;
    fn remove(&self, k: K) -> Result<(), E>;
    fn list(&self) -> Result<Vec<PathBuf>, E>;
}
