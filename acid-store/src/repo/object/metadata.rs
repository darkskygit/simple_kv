/*
 * Copyright 2019-2020 Wren Powell
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use super::config::RepositoryConfig;
use super::encryption::{KeySalt, ResourceLimit};
use super::{Compression, Encryption};

/// Metadata for a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryMetadata {
    /// The unique ID of this repository.
    pub id: Uuid,

    /// The number of bits that define a chunk boundary.
    ///
    /// The average size of a chunk will be 2^`chunker_bits` bytes.
    pub chunker_bits: u32,

    /// The compression method being used in this repository.
    pub compression: Compression,

    /// The encryption method being used in this repository.
    pub encryption: Encryption,

    /// The maximum amount of memory the key derivation function will use in bytes.
    pub memory_limit: ResourceLimit,

    /// The maximum number of computations the key derivation function will perform.
    pub operations_limit: ResourceLimit,

    /// The master encryption key encrypted with the user's password.
    pub master_key: Vec<u8>,

    /// The salt used to derive a key from the user's password.
    pub salt: KeySalt,

    /// The ID of the chunk which stores the repository's header.
    pub header: Uuid,

    /// The time this repository was created.
    pub creation_time: SystemTime,
}

impl RepositoryMetadata {
    /// Create a `RepositoryInfo` using the metadata in this struct.
    pub fn to_info(&self) -> RepositoryInfo {
        RepositoryInfo {
            id: self.id,
            config: RepositoryConfig {
                chunker_bits: self.chunker_bits,
                compression: self.compression,
                encryption: self.encryption,
                memory_limit: self.memory_limit,
                operations_limit: self.operations_limit,
            },
            created: self.creation_time,
        }
    }
}

/// Information about a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryInfo {
    id: Uuid,
    config: RepositoryConfig,
    created: SystemTime,
}

impl RepositoryInfo {
    /// The unique ID for this repository.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// The configuration used to create this repository.
    pub fn config(&self) -> &RepositoryConfig {
        &self.config
    }

    /// The time this repository was created.
    pub fn created(&self) -> SystemTime {
        self.created
    }
}

/// Statistics about a repository.
pub struct RepositoryStats {
    pub(super) apparent_size: u64,
    pub(super) actual_size: u64,
}

impl RepositoryStats {
    /// The combined size of all the objects in the repository.
    ///
    /// This may be larger than the `actual_size` due to deduplication and compression.
    pub fn apparent_size(&self) -> u64 {
        self.apparent_size
    }

    /// The total amount of space used by all the objects in the repository.
    ///
    /// This is an approximation of how much storage space is being used on the underlying data
    /// store.
    pub fn actual_size(&self) -> u64 {
        self.actual_size
    }
}
