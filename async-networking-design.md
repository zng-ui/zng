# Networking Tasks

## Should we encourage using one of the async crates?

Networking can use the OS async to work, it is not just a thread sleeping on a blocked operation,
like file read, both `async-std` and `smol` end-up using https://github.com/smol-rs/async-io and
that crate starts a thread that uses https://docs.rs/polling/2.1.0/polling/ to use the OS async.

So are they just a wrapper? They don't even do HTTP, just TCP/UDP.

## TODO

* Research HTTP client crates, do they use one of the general async crates?
* https://docs.rs/surf/2.2.0/surf/
* https://crates.io/crates/curl