# Services TODO

* Make a proc-macro that derives the service pattern from a normal looking struct.

* OR we can derive from a public `Foo` from `FooService`.

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

#[service]
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
pub(crate) struct FooService {
    data: bool,
}
impl Foo {
    fn __app_local__() -> &'static crate::app::AppLocal<FooService> {       
        app_local! {
            __FOO__: FooService = FooService::new();
        }
        &__FOO__
    }
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

impl FooService {
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
        Self::__app_local__().read().data()
    }

    /// Foo action.
    pub fn action(&self, r: bool) {
        Self::__app_local__().write().action(r);
    }

    #[allow(unused)]
    fn private_mtd(&mut self) { 
        Self::__app_local__().write().private_mtd(self);
    }
}
```