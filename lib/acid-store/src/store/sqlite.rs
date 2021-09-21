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

#![cfg(feature = "store-sqlite")]

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use lazy_static::lazy_static;

use crate::store::common::{DataStore, OpenOption, OpenStore};

lazy_static! {
    /// A UUID which acts as the version ID of the store format.
    static ref CURRENT_VERSION: Uuid =
        Uuid::parse_str("08d14eb8-4156-11ea-8ec7-a31cc3dfe2e4").unwrap();
}

/// A `DataStore` which stores data in a SQLite database.
#[derive(Debug)]
pub struct SqliteStore {
    /// The connection to the SQLite database.
    connection: Connection,
}

impl SqliteStore {
    /// Create a new `SqliteStore` at the given `path`.
    fn create_new(path: impl AsRef<Path>) -> crate::Result<Self> {
        if path.as_ref().exists() {
            return Err(crate::Error::AlreadyExists);
        }

        let connection = Connection::open(&path).map_err(anyhow::Error::from)?;

        connection
            .execute_batch(
                r#"
                    CREATE TABLE Blocks (
                        uuid BLOB PRIMARY KEY,
                        data BLOB NOT NULL
                    );
                    
                    CREATE TABLE Metadata (
                        key TEXT PRIMARY KEY,
                        value BLOB NOT NULL
                    );
                "#,
            )
            .map_err(anyhow::Error::from)?;

        connection
            .execute(
                r#"
                    INSERT INTO Metadata (key, value)
                    VALUES ('version', ?1);
                "#,
                params![&CURRENT_VERSION.as_bytes()[..]],
            )
            .map_err(anyhow::Error::from)?;

        Ok(SqliteStore { connection })
    }

    /// Open an existing `SqliteStore` at the given `path`.
    fn open_existing(path: impl AsRef<Path>) -> crate::Result<Self> {
        if !path.as_ref().is_file() {
            return Err(crate::Error::NotFound);
        }

        let connection = Connection::open(&path).map_err(anyhow::Error::from)?;

        let version_bytes: Vec<u8> = connection
            .query_row(
                r#"
                    SELECT value FROM Metadata
                    WHERE key = 'version';
                "#,
                params![],
                |row| row.get(0),
            )
            .map_err(|_| crate::Error::UnsupportedFormat)?;
        let version = Uuid::from_slice(version_bytes.as_slice())
            .map_err(|_| crate::Error::UnsupportedFormat)?;

        if version != *CURRENT_VERSION {
            return Err(crate::Error::UnsupportedFormat);
        }

        Ok(SqliteStore { connection })
    }
}

impl OpenStore for SqliteStore {
    type Config = PathBuf;

    fn open(config: Self::Config, options: OpenOption) -> crate::Result<Self>
    where
        Self: Sized,
    {
        if options.contains(OpenOption::CREATE_NEW) {
            Self::create_new(config)
        } else if options.contains(OpenOption::CREATE) && !config.exists() {
            Self::create_new(config)
        } else {
            let store = Self::open_existing(config)?;

            if options.contains(OpenOption::TRUNCATE) {
                store
                    .connection
                    .execute_batch(
                        r#"
                            DROP TABLE Blocks;
                            DROP TABLE Metadata;
                        "#,
                    )
                    .map_err(anyhow::Error::from)?;
            }

            Ok(store)
        }
    }
}

impl DataStore for SqliteStore {
    type Error = rusqlite::Error;

    fn write_block(&mut self, id: Uuid, data: &[u8]) -> Result<(), Self::Error> {
        self.connection.execute(
            r#"
                REPLACE INTO Blocks (uuid, data)
                VALUES (?1, ?2);
            "#,
            params![&id.as_bytes()[..], data],
        )?;

        Ok(())
    }

    fn read_block(&mut self, id: Uuid) -> Result<Option<Vec<u8>>, Self::Error> {
        self.connection
            .query_row(
                r#"
                    SELECT data FROM Blocks
                    WHERE uuid = ?1;
                "#,
                params![&id.as_bytes()[..]],
                |row| row.get(0),
            )
            .optional()
    }

    fn remove_block(&mut self, id: Uuid) -> Result<(), Self::Error> {
        self.connection.execute(
            r#"
                DELETE FROM Blocks
                WHERE uuid = ?1;
            "#,
            params![&id.as_bytes()[..]],
        )?;

        Ok(())
    }

    fn list_blocks(&mut self) -> Result<Vec<Uuid>, Self::Error> {
        let mut statement = self.connection.prepare(r#"SELECT uuid FROM Blocks;"#)?;

        let result = statement
            .query_map(params![], |row| {
                row.get(0).map(|bytes: Vec<u8>| {
                    Uuid::from_slice(bytes.as_slice()).expect("Could not parse UUID.")
                })
            })?
            .collect::<Result<Vec<Uuid>, _>>()?;

        Ok(result)
    }
}
