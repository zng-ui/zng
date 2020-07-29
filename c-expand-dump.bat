@echo off
IF [%1]==[] (
    cargo +nightly rustc --profile=check -- -Zunstable-options --pretty=expanded > dump.rs
    @echo expand ^*.rs ^> dump.rs
) ELSE (
    cargo +nightly rustc  --example %1 --profile=check -- -Zunstable-options --pretty=expanded > dump.rs
    @echo expand %1.rs ^> dump.rs
)