# Better HTTP Client

After examining `surf` there are some limitations, it does not supports cookies and it depends on to many other crates.

A better replacement is `isahc` that is the default backend for `surf` and can be used directly.

Evaluate if cookies should be enabled by default. (https://docs.rs/isahc/1.4.0/isahc/config/trait.Configurable.html#tymethod.cookie_jar)