use std::{
    any::{Any, TypeId},
    collections::HashMap,
    hash::Hash,
    process::Output,
    sync::{Arc, Weak},
};

use dashmap::DashMap;

pub struct AssetCache {
    cells: DashMap<TypeId, Box<dyn Any>>,
}

impl AssetCache {
    fn get<K: AssetKey>(&self, key: K) -> Arc<K::Output> {
        let mut cell = self.cells.entry(TypeId::of::<K>()).or_insert_with(|| {
            Box::new(AssetCell::<K> {
                loaded: HashMap::new(),
            })
        });

        let cell = cell.downcast_mut::<AssetCell<K>>().unwrap();
        cell.get(key)
    }
}

pub struct AssetCell<K: AssetKey> {
    loaded: HashMap<K, Weak<K::Output>>,
}

impl<K: AssetKey> AssetCell<K> {
    pub fn get(&mut self, key: K) -> Arc<K::Output> {
        if let Some(value) = self.loaded.get(&key).and_then(|v| v.upgrade()) {
            value
        } else {
            let value = key.load();
            let value = Arc::new(value);
            self.loaded.insert(key, Arc::downgrade(&value));
            value
        }
    }
}

pub trait AssetKey: 'static + Send + Sync + Hash + Eq {
    type Output;
    fn load(&self) -> Self::Output;
}
