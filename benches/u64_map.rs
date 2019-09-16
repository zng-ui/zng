#[macro_use]
extern crate bencher;

use bencher::{black_box, Bencher};
use fnv::FnvHashMap;
use std::collections::{BTreeMap, HashMap};

const COUNT: u64 = 1000;

fn get_hashmap(bench: &mut Bencher) {
    let map: HashMap<u64, &'static str> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(0);
    bench.iter(|| map.get(&idx))
}

fn get_fnv_hashmap(bench: &mut Bencher) {
    let map: FnvHashMap<u64, &'static str> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(0);
    bench.iter(|| map.get(&idx))
}

fn get_btreemap(bench: &mut Bencher) {
    let map: BTreeMap<u64, &'static str> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(0);
    bench.iter(|| map.get(&idx))
}

fn get_binary_search(bench: &mut Bencher) {
    let map: BinaryMap<u64, &'static str> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(0);
    bench.iter(|| map.get(&idx))
}

fn get_linear_worst_case(bench: &mut Bencher) {
    let map: Vec<(u64, &'static str)> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(COUNT + 1);
    bench.iter(|| map.iter().rev().find(|&(key, _)| key == &idx));
}

fn get_linear_best_case(bench: &mut Bencher) {
    let map: Vec<(u64, &'static str)> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(COUNT - 1);
    bench.iter(|| map.iter().rev().find(|&(key, _)| key == &idx));
}

fn set_hashmap(bench: &mut Bencher) {
    let mut map: HashMap<u64, &'static str> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(0);
    bench.iter(|| {
        let old = map.insert(idx, "new");

        black_box("work");

        if let Some(old) = old {
            map.insert(idx, old);
        } else {
            map.remove(&idx);
        }
    })
}

fn set_fnv_hashmap(bench: &mut Bencher) {
    let mut map: FnvHashMap<u64, &'static str> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(0);
    bench.iter(|| {
        let old = map.insert(idx, "new");

        black_box("work");

        if let Some(old) = old {
            map.insert(idx, old);
        } else {
            map.remove(&idx);
        }
    })
}

fn set_btreemap(bench: &mut Bencher) {
    let mut map: BTreeMap<u64, &'static str> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(0);
    bench.iter(|| {
        let old = map.insert(idx, "new");

        black_box("work");

        if let Some(old) = old {
            map.insert(idx, old);
        } else {
            map.remove(&idx);
        }
    })
}

fn set_binary_search(bench: &mut Bencher) {
    let mut map: BinaryMap<u64, &'static str> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(0);
    bench.iter(|| {
        let old = map.insert(idx, "new");

        black_box("work");

        if let Some(old) = old {
            map.insert(idx, old);
        } else {
            map.remove(&idx);
        }
    })
}

fn set_linear(bench: &mut Bencher) {
    let mut map: Vec<(u64, &'static str)> = (0..COUNT).into_iter().map(|i| (i, "value")).collect();
    let idx = black_box(0);
    bench.iter(|| {
        map.push((idx, "new"));
        black_box("work");
        map.truncate(map.len() - 1);
    });
}

benchmark_group!(
    benches,
    get_hashmap,
    get_fnv_hashmap,
    get_btreemap,
    get_binary_search,
    get_linear_worst_case,
    get_linear_best_case,
    set_hashmap,
    set_fnv_hashmap,
    set_btreemap,
    set_binary_search,
    set_linear
);
benchmark_main!(benches);

struct BinaryMap<TKey: Ord, TValue> {
    data: Vec<(TKey, TValue)>,
}

impl<TKey: Ord, TValue> std::iter::FromIterator<(TKey, TValue)> for BinaryMap<TKey, TValue> {
    fn from_iter<I: IntoIterator<Item = (TKey, TValue)>>(iter: I) -> Self {
        let mut data: Vec<(TKey, TValue)> = iter.into_iter().collect();
        data.sort_by(|a, b| a.0.cmp(&b.0));
        BinaryMap { data }
    }
}

impl<TKey: Ord, TValue> BinaryMap<TKey, TValue> {
    fn search(&self, key: &TKey) -> Result<usize, usize> {
        self.data.binary_search_by(|(k, _)| k.cmp(&key))
    }

    pub fn get(&self, key: &TKey) -> Option<&TValue> {
        self.search(key).map(|i| &unsafe { self.data.get_unchecked(i) }.1).ok()
    }

    pub fn insert(&mut self, key: TKey, value: TValue) -> Option<TValue> {
        match self.search(&key) {
            Ok(i) => Some(std::mem::replace(unsafe { self.data.get_unchecked_mut(i) }, (key, value)).1),
            Err(i) => {
                self.data.insert(i, (key, value));
                None
            }
        }
    }

    pub fn remove(&mut self, key: &TKey) -> Option<TValue> {
        if let Ok(i) = self.search(&key) {
            Some(self.data.remove(i).1)
        } else {
            None
        }
    }
}
