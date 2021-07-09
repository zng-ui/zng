# File Tasks

The existing async libraries try to mimic the `std::io::File`, we could provide a different concept,
a channel that "sends" to a `Write` implementer.

```rust
// tasks.rs
pub fn write<W: std::io::Write>(sync: W) -> (Sender<u8>, impl Future<Output=std::io::Result<()>>) {
    todo!()
}
```