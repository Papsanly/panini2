use std::{collections::HashMap, hash::Hash};

pub trait GroupBy<K, V> {
    fn group_by(self, key_fn: impl Fn(&V) -> K) -> HashMap<K, Vec<V>>;
}

impl<K: Hash + Eq, V: Clone, I: Iterator<Item = V>> GroupBy<K, V> for I {
    fn group_by(self, key_fn: impl Fn(&V) -> K) -> HashMap<K, Vec<V>> {
        let mut res: HashMap<K, Vec<V>> = HashMap::new();
        for item in self {
            let key = key_fn(&item);
            res.entry(key).or_default().push(item);
        }

        res
    }
}
