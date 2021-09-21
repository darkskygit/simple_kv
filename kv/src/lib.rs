#[cfg(feature = "acid_kv")]
mod acid_impl;
mod kv;
#[cfg(feature = "sled_kv")]
mod sled_impl;
#[cfg(feature = "zbox_kv")]
mod zbox_impl;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub use crate::kv::{KVBucket, KV};
#[cfg(feature = "acid_kv")]
pub use acid_impl::{AcidError, AcidKV, AcidKVBucket};
#[cfg(feature = "sled_kv")]
pub use sled_impl::{SledKV, SledKVBucket};
#[cfg(feature = "zbox_kv")]
pub use zbox_impl::{Repo, ZboxError, ZboxKV, ZboxKVBucket};

fn get_path_string<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().to_str().unwrap_or_default().into()
}

pub fn kv_init() {
    #[cfg(all(not(feature = "acid_kv"), feature = "zbox_kv"))]
    zbox::init_env();
}
