@echo off
IF [%1]==[] (
    cargo +nightly rustc --profile=check -- -Zunstable-options --pretty=expanded > dump.rs
    @echo expand ^*.rs ^> dump.rs
) ELSE (
    cargo +nightly rustc  --example %* --profile=check -- -Zunstable-options --pretty=expanded > dump.rs
    @echo expand %*.rs ^> dump.rs
)