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
pub fn any_one() {
    let r = async_test(async { any!(async { true }).await });

    assert!(r);
}

#[test]
pub fn any_nine() {
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
}

#[test]
pub fn run_wake_immediately() {
    async_test(async {
        run(async {
            yield_now().await;
        })
        .await;
    });
}

#[test]
pub fn run_panic_handling() {
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
pub fn run_panic_handling_parallel() {
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
