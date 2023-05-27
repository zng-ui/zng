# Localization TODO

* Fluent crate does not provide any built-in functions.
    - Specs don't say they are optional.
    - See: https://github.com/projectfluent/fluent-rs/issues/313

* Optimize.
    - `format_fallback` does multiple allocations just to get inputs for the formatter.
    - It is possible to implement something that only allocates the result string?
    - See https://github.com/projectfluent/fluent-rs/issues/319
    - Resources are held in a `String`, even if the source is a `&'static str`.

* Implement a better localization menu for the example.
    - Button in the corner, show all lang preferences.
    - Pop-up that allows selecting all lang preferences.
        - Two lists?