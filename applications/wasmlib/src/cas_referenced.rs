use core::marker::PhantomData;

use crate::{cas_get, cas_put, KeyHandle, CASHandle, read};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_cbor::{de::from_mut_slice, Result as CBORResult};
use alloc::rc::Rc;

struct CASObject<T: Serialize+DeserializeOwned> {
    data: T
    links: Vec<Rc<KeyHandle>>
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CASReferenced<T> {
    handle: Rc<KeyHandle>,
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
    pub fn get(&self) -> CBORResult<CASObject<T>> {
        let mut data_handle = cas_get(&self.handle).expect("failed to get handle");
        let mut data = read(&data_handle);

        let de_data = from_mut_slice(&mut data)?;
        Ok(CASObject {
            data: de_data,
            links: vec![],
        })
    }

    pub fn put(data: T, links: Vec<&KeyHandle>) -> CASReferenced<T> {
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

    pub fn from_handle(handle: Rc<KeyHandle>) -> CASReferenced<T> {
        CASReferenced {
            handle,
            _marker: PhantomData,
        }
    }
}
