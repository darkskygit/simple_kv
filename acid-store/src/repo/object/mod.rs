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

pub use self::compression::Compression;
pub use self::config::RepositoryConfig;
pub use self::encryption::{Encryption, ResourceLimit};
pub use self::header::Key;
pub use self::lock::LockStrategy;
pub use self::metadata::{RepositoryInfo, RepositoryStats};
pub use self::object::{ContentId, Object, ReadOnlyObject};
pub use self::open_repo::OpenRepo;
pub use self::repository::ObjectRepository;

mod chunk_store;
mod chunking;
mod compression;
mod config;
mod encryption;
mod header;
mod lock;
mod metadata;
mod object;
mod open_repo;
mod repository;
mod state;
