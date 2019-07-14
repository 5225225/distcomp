use crate::data_structures::log::Log;
use crate::Journal;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
enum KeyValueStoreOperation<TKey: Clone, TValue: Clone> {
    Insert { key: TKey, value: TValue },
    Clear,
    Remove { key: TKey },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyValueStore<TKey: Eq + std::hash::Hash + Clone, TValue: Clone> {
    #[serde(skip)]
    inner: HashMap<TKey, TValue>,
    #[serde(skip)]
    was_init: bool,
    log: Log<KeyValueStoreOperation<TKey, TValue>>,
}

impl<
        TKey: Eq + std::hash::Hash + Clone + Serialize + DeserializeOwned,
        TValue: Clone + Serialize + DeserializeOwned,
    > Default for KeyValueStore<TKey, TValue>
{
    fn default() -> Self {
        Self {
            inner: HashMap::new(),
            was_init: true,
            log: Log::new(),
        }
    }
}

impl<
        TKey: Eq + std::hash::Hash + Serialize + DeserializeOwned + Clone,
        TValue: Serialize + DeserializeOwned + Clone,
    > KeyValueStore<TKey, TValue>
{
    pub fn init<J: Journal>(&mut self, journal: &J) {
        if !self.was_init {
            for oper in self.log.forward_list(journal) {
                self.internal_do_oper(oper);
            }
            self.was_init = true;
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    fn assert_init(&self) {
        if !self.was_init {
            panic!("Tried to do an operation on an uninitalised log. Please call init first.")
        }
    }

    fn do_oper<J: Journal>(&mut self, journal: &J, oper: KeyValueStoreOperation<TKey, TValue>) {
        self.init(journal);

        self.internal_do_oper(oper.clone());

        self.log = self.log.push(journal, oper);
    }

    fn internal_do_oper(&mut self, oper: KeyValueStoreOperation<TKey, TValue>) {
        use KeyValueStoreOperation::*;
        match oper {
            Insert { key, value } => {
                self.inner.insert(key, value);
            }
            Clear => {
                self.inner.clear();
            }
            Remove { key } => {
                self.inner.remove(&key);
            }
        }
    }

    pub fn inner(&self) -> &HashMap<TKey, TValue> {
        self.assert_init();

        &self.inner
    }

    pub fn insert<J: Journal>(&mut self, journal: &J, key: TKey, value: TValue) {
        self.init(journal);

        self.do_oper(journal, KeyValueStoreOperation::Insert { key, value });
    }

    pub fn clear<J: Journal>(&mut self, journal: &J) {
        self.init(journal);

        self.do_oper(journal, KeyValueStoreOperation::Clear);
    }

    pub fn remove<J: Journal>(&mut self, journal: &J, key: TKey) {
        self.init(journal);

        self.do_oper(journal, KeyValueStoreOperation::Remove { key });
    }
}
