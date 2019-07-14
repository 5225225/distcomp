use crate::{CASReferenced, Journal};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Node<T> {
    parent: Option<CASReferenced<Node<T>>>,
    data: T,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Log<T> {
    head: Option<CASReferenced<Node<T>>>,
}

impl<T> Clone for Log<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Log<T> {}

impl<T: Serialize + DeserializeOwned> Log<T> {
    pub fn new() -> Self {
        Self { head: None }
    }

    pub fn push<J: Journal>(self, journal: &J, data: T) -> Self {
        match self.head {
            None => Self {
                head: Some(CASReferenced::put(journal, &Node { parent: None, data })),
            },
            Some(head) => Self {
                head: Some(CASReferenced::put(
                    journal,
                    &Node {
                        parent: Some(head),
                        data,
                    },
                )),
            },
        }
    }

    pub fn walk_back<J: Journal, F: FnMut(T)>(&self, journal: &J, mut callback: F) {
        let mut curr = &self.head;
        let mut got;

        while let Some(c) = curr {
            got = c.get(journal).unwrap();
            callback(got.data);
            curr = &got.parent;
        }
    }

    pub fn forward_list<J: Journal>(&self, journal: &J) -> Vec<T> {
        let mut vec = Vec::new();

        self.walk_back(journal, |x| vec.push(x));

        vec.reverse();

        vec
    }
}
