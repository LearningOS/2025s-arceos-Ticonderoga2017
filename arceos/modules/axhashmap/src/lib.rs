#![no_std]

use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash, Hasher};

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;


pub struct SimpleHasher {
    state: u64,
}

impl SimpleHasher {
    fn new() -> Self {
        let seed = axhal::misc::random() as u64;
        Self { state: seed }
    }
}

impl Hasher for SimpleHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state = self.state.wrapping_mul(31).wrapping_add(b as u64);
        }
    }
}

pub struct SimpleHasherBuilder;

impl BuildHasher for SimpleHasherBuilder {
    type Hasher = SimpleHasher;

    fn build_hasher(&self) -> SimpleHasher {
        SimpleHasher::new()
    }
}

pub struct AxHashMap<K, V, S = SimpleHasherBuilder> {
    buckets: Vec<BTreeMap<u64, (K, V)>>,
    hash_builder: S,
    len: usize,
}

impl<K, V> AxHashMap<K, V, SimpleHasherBuilder> 
where 
    K: Hash + Eq,
{
    pub fn new() -> Self {
        Self::with_capacity(16)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let bucket_count = capacity.next_power_of_two();
        let mut buckets = Vec::with_capacity(bucket_count);
        for _ in 0..bucket_count {
            buckets.push(BTreeMap::new());
        }
        Self {
            buckets,
            hash_builder: SimpleHasherBuilder,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn hash<Q: ?Sized>(&self, key: &Q) -> u64 
    where 
        Q: Hash,
    {
        let mut hasher = self.hash_builder.build_hasher();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn bucket_index(&self, hash: u64) -> usize {
        (hash as usize) & (self.buckets.len() - 1)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where 
        K: Hash + Eq
    {
        let hash = self.hash(&key);
        let bucket_idx = self.bucket_index(hash);
        let bucket = &mut self.buckets[bucket_idx];

        let mut found = None;

        for (stored_hash, (ref stored_key, _)) in bucket.iter() {
            if *stored_hash == hash && *stored_key == key {
                found = Some(*stored_hash);
                break;
                // if let Some((_, old_value)) = bucket.remove(stored_hash) {
                    // bucket.insert(hash, (key, value));
                    // return Some(old_value);
                // }
            }
        }
        if let Some(stored_hash) = found {
            if let Some((_, old_value)) = bucket.remove(&stored_hash) {
                bucket.insert(hash, (key, value));
                return Some(old_value);
            }
        }

        bucket.insert(hash, (key, value));
        self.len += 1;
        None
    }

    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V> 
    where 
        K: Borrow<Q>,
        Q: Hash + Eq
    {
        let hash = self.hash(key);
        let bucket_idx = self.bucket_index(hash);

        let bucket = &self.buckets[bucket_idx];

        for (stored_hash, (ref stored_key, ref value)) in bucket.iter() {
            if *stored_hash == hash && stored_key.borrow() == key {
                return Some(value);
            }
        }

        None
    }

    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V> 
    where  
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let hash = self.hash(key);
        let bucket_idx = self.bucket_index(hash);
        
        let bucket = &mut self.buckets[bucket_idx];
        let mut found_hash = None;

        for (stored_hash, (ref stored_key, _)) in bucket.iter() {
            if *stored_hash == hash && stored_key.borrow() == key {
                found_hash = Some(*stored_hash);
                break;
            }
        }

        if let Some(found_hash) = found_hash {
            if let Some((_, value)) = bucket.remove(&found_hash) {
                self.len -= 1;
                return Some(value);
            }
        }

        None
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            hashmap: self,
            bucket_idx: 0,
            bucket_iter: None,
            bucket_len: self.len,
        }
    }
}

pub struct Iter<'a, K, V> {
    hashmap: &'a AxHashMap<K, V, SimpleHasherBuilder>,
    bucket_idx: usize,
    bucket_iter: Option<alloc::collections::btree_map::Iter<'a, u64, (K, V)>>,
    bucket_len: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);    
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = &mut self.bucket_iter {
            if let Some((_, (k, v))) = iter.next() {
                return Some((k, v));
            }
        }

        while self.bucket_idx < self.hashmap.buckets.len() {
            let bucket = &self.hashmap.buckets[self.bucket_idx];
            self.bucket_idx += 1;

            if !bucket.is_empty() {
                self.bucket_iter = Some(bucket.iter());
                if let Some((_, (k, v))) = self.bucket_iter.as_mut().unwrap().next() {
                    return Some((k, v));
                }
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.bucket_len))
    }

}
