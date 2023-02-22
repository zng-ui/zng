# Services TODO

* Make a proc-macro that derives the service pattern from a normal looking struct.

```rust
/// Foo service.
/// 
/// # Examples
/// 
/// ```
/// FOO.action();
/// ```
#[service(FOO)]
pub struct Foo {
    data: bool,
}

#[service(FOO)]
impl Foo {
    fn new() -> Self {
        Foo {
            data: false,
        }
    }

    /// Foo data.
    pub fn data(&self) -> bool {
        self.data
    }

    /// Foo action.
    pub fn action(&mut self, r: bool) {
        self.data = r;
        self.private_mtd();
    }

    fn private_mtd(&mut self) { }
}
```

Expands too:

```rust
pub(crate) struct __Foo_Service__ {
    data: bool,
}
app_local! {
    pub(crate) static __FOO_SERVICE__: __Foo_Service__ = __Foo_Service__::new();
}

/// Foo service.
/// 
/// # Examples
/// 
/// ```
/// FOO.action();
/// ```
pub struct Foo { }

/// Instance of [`Foo`] for the current app.
pub static FOO: Foo = Foo { };

impl __Foo_Service__ {
    fn new() -> Self {
        Foo {
            data: false,
        }
    }

    pub(crate) fn data(&self) -> bool {
        self.data
    }

    pub(crate) fn action(&mut self, r: bool) {
        self.data = r;
        self.private_mtd();
    }

    fn private_mtd(&mut self) { }
}
impl Foo {
    /// Foo data.
    pub fn data(&self) -> bool {
        __FOO_SERVICE__.read().data()
    }

    /// Foo action.
    pub fn action(&self, r: bool) {
        __FOO_SERVICE__.write().action(r);
    }
}
```