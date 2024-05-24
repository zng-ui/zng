<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.

Cargo extension for Zng project management. Create a new project from templates, collect localization strings, package
the application for distribution.

# Installation

```console
cargo install cargo-zng
```

# Usage

Commands overview:

```console
# cargo zng --help

Zng project manager.

Usage: cargo zng <COMMAND>

Commands:
  l10n  Localization text scraper
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

```

## `l10n`

```console
# cargo zng l10n --help

Localization text scraper

See the docs for `l10n!` for more details about the expected format.

Usage: cargo zng l10n [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>
          Rust files glob

  <OUTPUT>
          Lang resources dir

Options:
  -m, --macros <MACROS>
          Custom l10n macro names, comma separated

          [default: ]

      --pseudo <PSEUDO>
          Pseudo Base name, empty to disable

          [default: pseudo]

      --pseudo-m <PSEUDO_M>
          Pseudo Mirrored name, empty to disable

          [default: pseudo-mirr]

      --pseudo-w <PSEUDO_W>
          Pseudo Wide name, empty to disable

          [default: pseudo-wide]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Also see [`zng::l10n::l10n!`] docs for more details about the expected format.

[`zng::l10n::l10n!`]: https://zng-ui.github.io/doc/zng/l10n/macro.l10n.html#scrap-template

## `new`

```console
# cargo zng new --help

Initialize a new repository from a Zng template repository

Usage: cargo zng new [OPTIONS] [VALUE]...

Arguments:
  [VALUE]...
          Set template values by position

          The first value for all templates is the app name.

          EXAMPLE

          cargo zng new "My App!" | creates a "my-app" project.

          cargo zng new "my_app"  | creates a "my_app" project.

Options:
  -t, --template <TEMPLATE>
          Zng template

          Can be `.git` URL or an `owner/repo` for a GitHub repository.

          Can also be an absolute path or `./path` to a local template directory.

          [default: zng-ui/zng-template]

  -s, --set [<SET>...]
          Set a template value

          Templates have a `.zng-template` file that defines the possible options.

  -k, --keys
          Show all possible values that can be set on the template

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Zng project generator is very simple, it does not use any template engine, just Rust's string replace in UTF-8 text files only.
The replacement keys are compilable, so template designers can build/check their template like a normal Rust project.

Template keys encode the format they provide, these are the current supported key cases:

* t-key-t — kebab-case
* T-KEY-T — UPPER-KEBAB-CASE
* t_key_t — snake_case
* T_KEY_T — UPPER_SNAKE_CASE
* T-Key-T — Train-Case
* t.key.t — lower case
* T.KEY.T — UPPER CASE
* T.Key.T — Title Case
* ttKeyTt — camelCase
* TtKeyTt — PascalCase
* {{key}} — Unchanged.

The values for each format (except {{key}}) are cleaned of chars that do not match this pattern
`[ascii_alphabetic][ascii_alphanumeric|'-'|'_'|' '|]*`. The case and separator conversions are applied to this
cleaned value.

The actual keys are declared by the template in a `.zng-template` file in the root of the template repository, they
are ascii alphabetic with >=3 lowercase chars.

Call `cargo zng new --keys` to show help for the template keys.

The default template has 3 keys:

* `app` — The app name, the Cargo package and crate names are derived from it. Every template first key must be this one.
* `org` — Used in `zng::env::init` as the 'organization' value.
* `qualifier` — Used in `zng::env::init` as the 'qualifier' value.

For an example input `cargo zng new "My App!" "My Org"` the template code:

```rust
// file: src/t_app_t_init.rs

pub fn init_t_app_t() {
    println!("init t-app-t");
    zng::env::init("{{qualifier}}", "{{org}}", "{{app}}");
}
```

Generates:

```rust
// file: src/my_app_init.rs

pub fn init_my_app() {
    println!("init my-app");
    zng::env::init("", "My Org", "My App!");
}
```

See [zng-ui/zng-template] for an example of templates.

[zng-ui/zng-template]: https://github.com/zng-ui/zng-template
