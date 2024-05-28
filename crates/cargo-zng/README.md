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

Usage: cargo-zng.exe <COMMAND>

Commands:
  new   Initialize a new repository from a Zng template repository
  l10n  Localization text scraper
  res   Build resources
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## `l10n`

Localization text scraper.

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

Initialize a new repository from a Zng template repository.

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

# `res`

Build resources

```console
# cargo zng res --help

Build resources

Walks SOURCE and delegates `.zr-{tool}` files to `cargo-zng-res-{tool}` executables and crates.   

Usage: cargo zng res [OPTIONS] [SOURCE] [TARGET]

Arguments:
  [SOURCE]
          Resources source dir

          [default: assets]

  [TARGET]
          Resources target dir

          This directory is wiped before each build.

          [default: target/assets]

Options:
      --pack
          Copy all static files to the target dir

      --tools <TOOLS>
          Search for `zng-res-{tool}` in this directory first

          [default: tools]

      --list
          Prints help for all tools available

      --tool-cache <TOOL_CACHE>
          Tool cache dir

          [default: target/assets.cache]

      --recursion-limit <RECURSION_LIMIT>
          Number of build passes allowed before final

          [default: 32]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

This subcommand can be used to build assets and package releases. It is very simple, you create
a resources directory tree as close as possible to the final resources structure, and place special
`.zr-{tool}` files on it that are calls to `cargo-zng-res-{tool}` crates or executables.

## Resource Build

The resource build follows these steps:

* The TARGET dir is wiped clean.
* The SOURCE dir is walked, matching directories are crated on TARGET, `.zr-*` tool requests are run.
* The TARGET dir is walked, any new `.zr-*` request generated by previous pass are run (request is removed after tool run).
  - This repeats until a pass does not find any `.zr-*` or the `--recursion_limit` is reached.
* Run all tools that requested `zng-res::on-final=` from a request that still exists.

## Tools

You can call `cargo zng res --list` to see help for all tools available. Tools are searched in this order:

* If a crate exists in `tools/cargo-zng-res-{tool}` executes it (with `--quiet` build).
* If a crate exists in `tools/cargo-zng-res` and it has a `src/bin/{tool}.rs` file executes it with `--bin {tool}`.
* If the tool is built in, executes it.
* If a `cargo-zng-res-{tool}[.exe]` is installed in the same directory as the running `cargo-zng[.exe]`, executes it.

### Authoring Tools

Tools are configured using environment variables:

* `ZR_SOURCE_DIR` — Resources directory that is being build.
* `ZR_TARGET_DIR` — Target directory where resources are bing built to.
* `ZR_CACHE_DIR` — Dir to use for intermediary data for the specific request. Keyed on the source dir, target dir, request file and request file content.
* `ZR_WORKSPACE_DIR` — Cargo workspace that is parent to the source dir. This is also the working dir (`current_dir`) set for the tool.
* `ZR_REQUEST` — Request file that called the tool.
* `ZR_TARGET` — Target file implied by the request file name. That is, the request filename without `.zr-{tool}` and in the equivalent target subdirectory.
* `ZR_FINAL` — Set to the args if the tool requested `zng-res::on-final={args}`.
* `ZR_HELP` — Print help text for `cargo zng res --list`. If this is set the other vars will not be set.

Tools can make requests to the resource builder by printing to stdout with prefix `zng-res::`.
Current supported requests:

* `zng-res::delegate` — Continue searching for a tool that can handle this request.
* `zng-res::warning={message}` — Prints the `{message}` as a warning.
* `zng-res::on-final={args}` — Subscribe to be called again with `ZR_FINAL={args}` after all tools have run.

If the tool fails the entire stderr is printed and the resource build fails.

A rebuild starts by removing the target dir and runs all tools again. If a tool task is potentially
slow is should cache results. The `ZNG_RES_CACHE` environment variable is set with a path to a directory 
where the tool can store intermediary files specific for this request. The cache dir is keyed to the 
`<SOURCE><TARGET><REQUEST>` and the request file content.

The tool working directory (`current_dir`) is always set to the Cargo workspace root. if the `<SOURCE>`
is not inside any Cargo project a warning is printed and the `<SOURCE>` is used as working directory.
