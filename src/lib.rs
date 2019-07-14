#![deny(clippy::all)]
#![deny(clippy::pedantic)]

use rusqlite::params;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::OptionalExtension;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sodiumoxide::crypto::hash::sha256;
use sodiumoxide::crypto::sign;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::marker::PhantomData;
use uuid::Uuid;

pub mod data_structures;

/// A 32 byte key type used to reference journal entries. Similar to a git commit.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JournalKey([u8; 32]);

impl fmt::Debug for JournalKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JournalKey(")?;

        for i in &self.0 {
            write!(f, "{:x}", i)?;
        }

        write!(f, ")")?;

        Ok(())
    }
}

impl FromSql for JournalKey {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        let b = value.as_blob()?;

        let r = <[u8; 32]>::try_from(b).map_err(|x| FromSqlError::Other(Box::new(x)));

        Ok(Self(r?))
    }
}

impl FromSql for CASKey {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        let b = value.as_blob()?;

        let r = <[u8; 32]>::try_from(b).map_err(|x| FromSqlError::Other(Box::new(x)));

        Ok(Self(r?))
    }
}

/// A key type used to wrap a [`sign::PublicKey`] to refer to a device.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DevicePublicKey(sign::PublicKey);

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CASReferenced<T> {
    key: CASKey,
    #[serde(skip)]
    _marker: PhantomData<T>,
}

impl<T> Clone for CASReferenced<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for CASReferenced<T> {}

impl<T: Serialize + DeserializeOwned> CASReferenced<T> {
    pub fn put<J: Journal>(journal: &J, data: &T) -> Self {
        let ser = serde_cbor::to_vec(data).unwrap();

        Self {
            key: journal.cas_put(ser),
            _marker: PhantomData,
        }
    }

    pub fn get<J: Journal>(&self, journal: &J) -> Option<T> {
        let data = journal.cas_get(self.key)?;

        serde_cbor::from_slice(&data).ok()?
    }
}

impl FromSql for DevicePublicKey {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        let b = value.as_blob()?;

        let k = sign::PublicKey::from_slice(b).unwrap();

        Ok(Self(k))
    }
}

impl FromSql for ApplicationId {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        let b = value.as_blob()?;

        let uuid: Uuid = Uuid::from_slice(b).unwrap();

        Ok(Self(uuid))
    }
}

pub trait Journal {
    fn settings_get(&self, key: &str) -> Option<Vec<u8>>;
    fn settings_set(&self, key: &str, value: &[u8]);

    fn pubkey(&self) -> DevicePublicKey {
        let key = self.settings_get("PublicKey").unwrap();
        DevicePublicKey(sign::PublicKey::from_slice(&key).unwrap())
    }

    fn privkey(&self) -> sign::SecretKey {
        let key = self.settings_get("PrivateKey").unwrap();
        sign::SecretKey::from_slice(&key).unwrap()
    }

    fn this_head(&self, application_id: ApplicationId) -> Option<JournalKey> {
        Some(*self.heads().get(&(application_id, self.pubkey()))?)
    }

    fn heads(&self) -> HashMap<(ApplicationId, DevicePublicKey), JournalKey>;
    fn update_head(&self, device: DevicePublicKey, appid: ApplicationId, key: JournalKey);

    fn get(&self, key: JournalKey) -> Option<JournalEntry>;
    fn put(&self, entry: JournalEntry, keypair: (sign::SecretKey, sign::PublicKey)) -> JournalKey;

    fn cas_get(&self, key: CASKey) -> Option<Vec<u8>>;
    fn cas_put(&self, data: Vec<u8>) -> CASKey;
    fn cas_list(&self) -> Vec<CASKey>;

    fn easy_cas_get<T: DeserializeOwned>(&self, key: CASKey) -> Option<T> {
        let data = self.cas_get(key)?;

        serde_cbor::from_slice(&data).ok()?
    }

    fn easy_cas_put<T: Serialize>(&self, data: &T) -> CASKey {
        let ser = serde_cbor::to_vec(&data).unwrap();

        self.cas_put(ser)
    }

    fn commit_self(&self, application_id: ApplicationId, new_state: CASKey) -> JournalKey {
        let head = self.heads().get(&(application_id, self.pubkey())).cloned();

        let mut parents = vec![];

        if let Some(head) = head {
            parents.push(head);
        }

        let entry = JournalEntry {
            parents,
            application_id,
            new_state,
        };

        let put_entry = self.put(entry, (self.privkey(), self.pubkey().0));

        self.update_head(self.pubkey(), application_id, put_entry);

        put_entry
    }

    fn update_state<T: Serialize>(&self, data: &T, appid: ApplicationId) -> (CASKey, JournalKey) {
        let entry = self.easy_cas_put(data);

        let key = self.commit_self(appid, entry);

        (entry, key)
    }

    fn get_state<T: DeserializeOwned>(&self, appid: ApplicationId) -> Option<(T, JournalEntry)> {
        let head = self.this_head(appid)?;

        let head_entry = self.get(head)?;

        let state = head_entry.new_state;

        Some((self.easy_cas_get::<T>(state)?, head_entry))
    }
}

#[derive(Debug)]
pub struct SqliteJournal {
    db: rusqlite::Connection,
}

impl SqliteJournal {
    pub fn new(path: &str) -> Self {
        let db = rusqlite::Connection::open(path).expect("failed to create database");

        db.set_prepared_statement_cache_capacity(32);

        db.execute_batch(
            "

        PRAGMA journal_mode=WAL;

        CREATE TABLE IF NOT EXISTS settings (
            id BLOB NOT NULL PRIMARY KEY,
            value BLOB NOT NULL
        );

        CREATE TABLE IF NOT EXISTS heads (
            application_id BLOB NOT NULL,
            device_id BLOB NOT NULL,
            entry_id BLOB NOT NULL,
            PRIMARY KEY (application_id, device_id)
        );

        CREATE TABLE IF NOT EXISTS entries (
            id BLOB NOT NULL PRIMARY KEY,
            inner BLOB NOT NULL
        );

        CREATE TABLE IF NOT EXISTS cas (
            id BLOB NOT NULL PRIMARY KEY,
            content BLOB NOT NULL
        );",
        )
        .unwrap();

        let journal = Self { db };

        if journal.settings_get("PrivateKey").is_none() {
            let (pubkey, privkey) = sign::gen_keypair();

            journal.settings_set("PublicKey", &pubkey[..]);
            journal.settings_set("PrivateKey", &privkey[..]);
        }

        journal
    }
}

impl Journal for SqliteJournal {
    fn settings_get(&self, key: &str) -> Option<Vec<u8>> {
        self.db
            .prepare_cached("SELECT value FROM settings WHERE id=?1")
            .unwrap()
            .query_row(params!(key), |row| row.get(0))
            .optional()
            .unwrap()
    }

    fn settings_set(&self, key: &str, value: &[u8]) {
        self.db
            .prepare_cached("INSERT INTO settings VALUES (?1, ?2)")
            .unwrap()
            .execute(params!(key, value))
            .unwrap();
    }

    fn heads(&self) -> HashMap<(ApplicationId, DevicePublicKey), JournalKey> {
        self.db
            .prepare_cached("SELECT application_id, device_id, entry_id FROM heads")
            .unwrap()
            .query_map(params!(), |row| {
                Ok(((row.get(0)?, row.get(1)?), row.get(2)?))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn update_head(&self, device: DevicePublicKey, appid: ApplicationId, key: JournalKey) {
        self.db
            .prepare_cached("INSERT OR REPLACE INTO heads VALUES (?, ?, ?)")
            .unwrap()
            .execute(params!(appid.0, &device.0[..], &key.0[..]))
            .unwrap();
    }

    fn get(&self, key: JournalKey) -> Option<JournalEntry> {
        let result: Vec<u8> = self
            .db
            .prepare_cached("SELECT inner FROM entries WHERE id = ?1")
            .unwrap()
            .query_row(params!(&key.0[..]), |row| row.get(0))
            .optional()
            .unwrap()?;

        let des: Signed = serde_cbor::from_slice(&result).unwrap();

        let inner_journal_entry = sign::verify(&des.inner_signed, &des.from).unwrap();

        let final_des: JournalEntry = serde_cbor::from_slice(&inner_journal_entry).unwrap();

        Some(final_des)
    }

    fn put(&self, entry: JournalEntry, keypair: (sign::SecretKey, sign::PublicKey)) -> JournalKey {
        let ser = serde_cbor::to_vec(&entry).unwrap();

        let signed = Signed {
            from: keypair.1,
            inner_signed: sign::sign(&ser, &keypair.0),
        };

        let signed_ser = serde_cbor::to_vec(&signed).unwrap();

        let digest = sha256::hash(&signed_ser);

        self.db
            .prepare_cached("INSERT OR IGNORE INTO entries VALUES (?1, ?2)")
            .unwrap()
            .execute(params!(&digest[..], &signed_ser))
            .unwrap();

        JournalKey(digest.as_ref().try_into().unwrap())
    }

    fn cas_get(&self, key: CASKey) -> Option<Vec<u8>> {
        self.db
            .prepare_cached("SELECT content FROM cas WHERE id = ?1")
            .unwrap()
            .query_row(params!(&key.0[..]), |row| row.get(0))
            .optional()
            .unwrap()
    }

    fn cas_put(&self, data: Vec<u8>) -> CASKey {
        let digest = sha256::hash(&data);

        self.db
            .prepare_cached("INSERT OR IGNORE INTO cas VALUES (?1, ?2)")
            .unwrap()
            .execute(params!(&digest[..], data))
            .unwrap();

        CASKey(digest.as_ref().try_into().unwrap())
    }

    fn cas_list(&self) -> Vec<CASKey> {
        self.db
            .prepare_cached("SELECT id FROM cas")
            .unwrap()
            .query_map(params!(), |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
struct Signed {
    from: sign::PublicKey,
    inner_signed: Vec<u8>,
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ApplicationId(pub Uuid);

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct JournalEntry {
    application_id: ApplicationId,
    new_state: CASKey,
    parents: Vec<JournalKey>,
}

#[derive(Copy, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct CASKey([u8; 32]);

impl fmt::Debug for CASKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CASKey(")?;

        for i in &self.0 {
            write!(f, "{:x}", i)?;
        }

        write!(f, ")")?;

        Ok(())
    }
}
