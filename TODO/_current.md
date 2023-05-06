# Text Edit

* Implement `accepts_return`.
* Implement cursor position.
* Implement selection.

# Localization

* Implement proc-macro.
* Add example file to Cargo.toml.

## Design

* Similar to `CONFIG`, maybe even backed by it.
    - Can we use a config backing store?
    - Need a "directory-db" mode, each file is a locale.
    - Review file watcher impl, may need to use a crate that uses the system API now.
* Use <https://docs.rs/fluent/> by default.
* Started `L10N` service.
    - Move to `text` module?
* Service provides variables with message object values already in the `LANG_VAR` context.
    - Variable updates when `LANG_VAR` changes or when the localization file changes.
* Formatting then uses `merge_var!` to also be dynamic.
    - Could make a macro that does this, can implement some tool that generates the initial localization file from this macro?
* If we are selecting a backend should we be a separate crate?
    - No, we need `l10n!` to be a proc-macro that validates the string, if we have a separate crate we will need two separate crates.
    - Every app should have localization, even if just the English text is provided, users can contribute their own localization.
    - We already need a separate crate for the scrapper code?
        - We don't. We are not even planing on using the scrapper on build, too expensive and not a good idea, should scrap only
          when a release is aumost done, so translators don't work on strings that endup removed or changed.

```rust
// all in one place, supports scrapper tools.
Text! {
    // l10n: docs for scrapper tool in previous line.
    txt = l10n!("op-result", "Found {$n} results.", n=count_var.clone()); // l10n: docs for scrapper same line.
}

// supports interpolation and is a proc-macro that validates.
let n = var(42);
Text! {
    txt = l10n!("op-result", "Found {$n} results.");
}

// format syntax is the Fluent Project syntax:
let n = var(42);
Text! {
    // l10n: $n is an integer, min 0.
    txt = l10n!("op-result", "Found {$n} {$n -> [one]result *[other]results}.");
}

// fluent attributes must be typed in for the scrapper tool:
let n = var(42);
Text! {
    txt = l10n!("op-result", "Found {$n} results.");
    tooltip = Tip!(Text!(l10n!("op-result.tip", "result count")));
}
// we don't support contextual id/attributes because if breaks the scrapper tool.
//  - the inlined declaration of default makes-up for this verbosity?
//  - FluentJS only supports attributes in the same element too?
```

* Do we have one localization data source per app, window, widget?
    - Can imagine needing a different data source per widget even.
    - Data source can be contextual, like the LANG, but how do we select what source to write in the scrapper?
    - Defined by comments?

```rust
// l10n-source: task-messages.$lang.flt

Window! {
    l10n_source = "task-messages.$lang.flt";
    title = l10n!("window.title");
}
```
    - all scrapped text get scrapped to file `"task-messages.template.flt"`.
    - comment sets the source file for all `l10n!` usages under it in the file?
    - seems very precarious.

* Other macros:
    - `l10n_txt!("id", "fmt")`, is scrapped and expands to `l10n!("id", "fmt").get()`.
    - `l10n_str!("id", "fmt")`, is scrapped and expands to `l10n!("id", "fmt").get().to_string()` or equivalent.
    - `l10n_panic!("id", "fmt")`, is scrapped and expands to `panic!("{}", l10n_txt!("id", "fmt"))`.
    - Scrapper can have a config for extra macros to match.