use crate::Key;
use core::marker::PhantomData;

use crate::{cas_get, cas_put};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_cbor::{de::from_mut_slice, Result as CBORResult};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(transparent)]
pub struct CASReferenced<T> {
    pub key: Key,
    _marker: PhantomData<T>,
}

enum WriteVecError {}

impl core::convert::Into<serde_cbor::error::Error> for WriteVecError {
    fn into(self) -> serde_cbor::error::Error {
        match self {}
    }
}

struct WriteVec(alloc::vec::Vec<u8>);

impl serde_cbor::ser::Write for WriteVec {
    type Error = WriteVecError;

    fn write_all(&mut self, buf: &[u8]) -> Result<(), WriteVecError> {
        self.0.extend_from_slice(buf);

        Ok(())
    }
}

impl<T: Serialize + DeserializeOwned> CASReferenced<T> {
    pub fn get(&self) -> CBORResult<T> {
        let mut data = cas_get(&self.key);

        from_mut_slice(&mut data)
    }

    pub fn put(data: T) -> CASReferenced<T> {
        let mut writer = WriteVec(alloc::vec::Vec::new());

        let mut ser = serde_cbor::Serializer::new(writer);

        data.serialize(&mut ser);

        let data = ser.into_inner().0;

        let key = cas_put(&data);

        CASReferenced {
            key,
            _marker: PhantomData,
        }
    }

    pub fn from_key(key: Key) -> CASReferenced<T> {
        CASReferenced {
            key,
            _marker: PhantomData,
        }
    }
}
