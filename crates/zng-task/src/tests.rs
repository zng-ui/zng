use std::time::Instant;

use rayon::prelude::*;

use super::*;
use zng_unit::TimeUnits;

#[track_caller]
fn async_test<F>(test: F) -> F::Output
where
    F: Future,
{
    block_on(with_deadline(test, 20.secs())).unwrap()
}

#[test]
fn any_one() {
    let r = async_test(async { any!(async { true }).await });

    assert!(r);
}

#[test]
fn any_nine() {
    let t = Instant::now();
    let one_s = 1.secs();
    let r = async_test(async {
        any!(
            async {
                deadline(one_s).await;
                1
            },
            async {
                deadline(one_s).await;
                2
            },
            async {
                deadline(one_s).await;
                3
            },
            async {
                deadline(one_s).await;
                4
            },
            async {
                deadline(one_s).await;
                5
            },
            async {
                deadline(one_s).await;
                6
            },
            async {
                deadline(one_s).await;
                7
            },
            async {
                deadline(one_s).await;
                8
            },
            async { 9 },
        )
        .await
    });

    assert_eq!(9, r);
    assert!(2.secs() > t.elapsed());
}

#[test]
fn run_wake_immediately() {
    async_test(async {
        run(async {
            yield_now().await;
        })
        .await;
    });
}

#[test]
fn run_panic_handling() {
    async_test(async {
        let r = run_catch(async {
            run(async {
                deadline(1.ms()).await;
                panic!("test panic")
            })
            .await;
        })
        .await;

        assert!(r.is_err());
    })
}

#[test]
fn run_panic_handling_parallel() {
    async_test(async {
        let r = run_catch(async {
            run(async {
                deadline(1.ms()).await;
                (0..100000).into_par_iter().for_each(|i| {
                    if i == 50005 {
                        panic!("test panic");
                    }
                });
            })
            .await;
        })
        .await;

        assert!(r.is_err());
    })
}

#[test]
fn fn_all() {
    let expected: Vec<_> = (0..20).collect();
    let tasks: Vec<_> = expected
        .iter()
        .map(|&i| async move {
            crate::deadline(((20 - i) * 50).ms()).await;
            i
        })
        .collect();

    let t = Instant::now();

    let results = async_test(async move { crate::all(tasks).await });

    assert_eq!(expected, results);
    assert!((30 * 50).ms() > t.elapsed())
}

#[test]
fn fn_all_ok_ok() {
    let expected: Vec<_> = (0..20).collect();
    let tasks: Vec<_> = expected
        .iter()
        .map(|&i| async move {
            crate::deadline(((20 - i) * 50).ms()).await;
            Ok::<_, String>(i)
        })
        .collect();

    let t = Instant::now();

    let results = async_test(async move { crate::all_ok(tasks).await }).unwrap();

    assert_eq!(expected, results);
    assert!((30 * 50).ms() > t.elapsed())
}

#[test]
fn fn_all_ok_err() {
    let expected: Vec<_> = (0..20).collect();
    let tasks: Vec<_> = expected
        .iter()
        .map(|&i| async move {
            crate::deadline(((20 - i) * 50).ms()).await;
            if i == 10 {
                return Err("error".to_owned());
            }
            Ok::<_, String>(i)
        })
        .collect();

    let t = Instant::now();

    let results = async_test(async move { crate::all_ok(tasks).await }).unwrap_err();

    assert_eq!("error", results);
    assert!((30 * 50).ms() > t.elapsed())
}

#[test]
fn fn_all_some_some() {
    let expected: Vec<_> = (0..20).collect();
    let tasks: Vec<_> = expected
        .iter()
        .map(|&i| async move {
            crate::deadline(((20 - i) * 50).ms()).await;
            Some(i)
        })
        .collect();

    let t = Instant::now();

    let results = async_test(async move { crate::all_some(tasks).await }).unwrap();

    assert_eq!(expected, results);
    assert!((30 * 50).ms() > t.elapsed())
}

#[test]
fn fn_all_some_none() {
    let expected: Vec<_> = (0..20).collect();
    let tasks: Vec<_> = expected
        .iter()
        .map(|&i| async move {
            crate::deadline(((20 - i) * 50).ms()).await;
            if i == 10 {
                return None;
            }
            Some(i)
        })
        .collect();

    let t = Instant::now();

    let results = async_test(async move { crate::all_some(tasks).await });

    assert!(results.is_none());
    assert!((30 * 50).ms() > t.elapsed())
}
