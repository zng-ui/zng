#[macro_use]
extern crate bencher;

use bencher::{black_box, Bencher};
use fnv::FnvHashMap;
use std::collections::{BTreeMap, HashMap};

const COUNT: u64 = 10;

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
    get_linear_worst_case,
    get_linear_best_case,
    set_hashmap,
    set_fnv_hashmap,
    set_btreemap,
    set_linear
);
benchmark_main!(benches);
