# Text Edit

* Implement `accepts_return`.
* Implement cursor position.
* Implement selection.

# WATCHER

* Test `sync`.
```rust
fn test_sync() -> impl UiNode {
    Toggle! {
        child = Text!("signal");
        style_fn = toggle::CheckStyle!();
        checked = WATCHER.sync(
            "F://Test/Temp/signal.txt",
            true,
            |r| {
                // try
                match (|| r?.parse())() {
                    Ok(b) => Some(b),
                    Err(e) => {
                        tracing::error!("{e}");
                        None
                    }
                }
            },
            |b, w| {
                // try
                if let Err(e) = (move || {
                    let mut w = w?;
                    w.write_fmt(format_args!("{b}"))?;
                    w.commit()
                })() {
                    tracing::error!("{e}");
                }
            }
        );
    }
}
```
* Use the new service in `CONFIG`.

# Localization

* Implement resource loader.
    - Can we use `CONFIG` as backing store?
    - Need a "directory-db" mode, a common pattern for apps is having each locale in a diferent file in the same dir.
    - Review file watcher impl, may need to use a crate that uses the system API now.
* Implement builder.
* Move `Lang` and lang related stuff to `l10n` module.
* Add variable args in example.
* Test "// l10n-source: test.$lang.flt" comments.

* Other macros:
    - `l10n_txt!("id", "fmt")`, is scrapped and expands to `l10n!("id", "fmt").get()`.
    - `l10n_str!("id", "fmt")`, is scrapped and expands to `l10n!("id", "fmt").get().to_string()` or equivalent.

# Other

* Review external crate re-exports.
    - We re-export `rayon` and `parking_lot`, are we the only ones to do this?
    - Users are expected to import the crate even if our API surface types form it?
* Implement something to only show one tooltip at a time.
* Fix `layer_remove_delay` not receiving `LAYER_REMOVE_REQUESTED_EVENT` after reinit.