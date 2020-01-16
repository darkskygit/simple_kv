mod kv;
mod zbox_impl;

use kv::{KVBucket, KV};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[cfg(features = "zbox_kv")]
pub use zbox_impl::{ZboxKV, ZboxKvBucket};

fn get_path_string<P: AsRef<Path>>(path: P) -> String {
    path.as_ref()
        .as_os_str()
        .to_os_string()
        .into_string()
        .unwrap_or_default()
}
