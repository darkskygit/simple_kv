#[cfg(feature = "acid_kv")]
mod acid_impl;
mod kv;
#[cfg(feature = "sled_kv")]
mod sled_impl;
#[cfg(feature = "zbox_kv")]
mod zbox_impl;

use kv::{KVBucket, KV};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[cfg(feature = "acid_kv")]
pub use acid_impl::{AcidError, AcidKV, AcidKVBucket};
#[cfg(feature = "sled_kv")]
pub use sled_impl::{SledKV, SledKVBucket};
#[cfg(feature = "zbox_kv")]
pub use zbox_impl::{Repo, ZboxError, ZboxKV, ZboxKVBucket};

fn get_path_string<P: AsRef<Path>>(path: P) -> String {
    path.as_ref()
        .as_os_str()
        .to_os_string()
        .into_string()
        .unwrap_or_default()
}

pub fn kv_init() {
    #[cfg(feature = "acid_kv")]
    acid_store::init();
    #[cfg(all(not(feature = "acid_kv"), feature = "zbox_kv"))]
    zbox::init_env();
}
