use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_cbor::Result as CBORResult;

use crate::cas_referenced::CASReferenced;

#[derive(Clone, Debug)]
pub struct Stack<T> {
    head: Option<CASReferenced<StackNode<T>>>,
}

#[derive(Clone, Debug)]
struct StackNode<T> {
    parent: Option<CASReferenced<StackNode<T>>>,
    data: T,
}

impl<T: Serialize + DeserializeOwned> Stack<T> {
    pub fn new() -> Self {
        Self { head: None }
    }

    pub fn push(self, data: T) -> Self {
        let node = StackNode {
            parent: self.head,
            data,
        };

        let put_node = CASReferenced::put(node);

        Stack {
            head: Some(put_node),
        }
    }

    pub fn walk_backwards(self, callback: &mut dyn FnMut(T)) -> CBORResult<()> {
        let mut curr_node = self.head;

        while let Some(node) = curr_node {
            let new_node = node.get()?;

            callback(new_node.data);

            curr_node = new_node.parent;
        }

        Ok(())
    }
}
