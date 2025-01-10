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

<!--do doc --readme do zng --help -->
```console
$ cargo zng --help

```

## `fmt`

Formats the code with `cargo fmt` and formats Zng macros and some other braced macros.

<!--do doc --readme do zng fmt --help -->
```console
$ cargo zng fmt --help

```

The formatter supports Zng macros and also attempts to format all braced macro contents 
like `foo! { <contents> }` by copying it into a temporary item `fn _fmt_item() { <contents> }` 
and trying `rustfmt`, if the contents cannot be formatted like this they are not touched.

### IDE Integration

You can configure Rust-Analyzer to use `cargo zng fmt --stdin` as your IDE formatter. 

In VsCode add this to the workspace config at `.vscode/settings.json`:

```json
"rust-analyzer.rustfmt.overrideCommand": [
    "cargo",
    "zng",
    "fmt",
    "--stdin"
],
```

Now Zng macros format with the format context action and command.

## `new`

Initialize a new repository from a Zng template repository.

<!--do doc --readme do zng new --help -->
```console
$ cargo zng new --help

```

The Zng project generator is very simple, it does not use any template engine, just Rust's string replace in UTF-8 text files only.
The replacement keys are valid crate/type names, so template designers can build/check their template like a normal Rust project.

Template keys encode the format they provide, these are the current supported key cases:

* t-key-t — kebab-case (cleaned)
* T-KEY-T — UPPER-KEBAB-CASE (cleaned)
* t_key_t — snake_case (cleaned)
* T_KEY_T — UPPER_SNAKE_CASE (cleaned)
* T-Key-T — Train-Case (cleaned)
* t.key.t — lower case
* T.KEY.T — UPPER CASE
* T.Key.T — Title Case
* ttKeyTt — camelCase (cleaned)
* TtKeyTt — PascalCase (cleaned)
* {{key}} — Unchanged
* f-key-f — Sanitized, otherwise unchanged
* f-Key-f — Title Case (sanitized)

Cleaned values only keep ascii alphabetic first char and ascii alphanumerics, ' ', '-' and '_' other chars.

Sanitized values are valid file names in all operating systems. Values in file names are automatically sanitized.

The actual keys are declared by the template in the `.zng-template/keys` file, they
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

### Ignore

The `.zng-template` directory is not included in the final template, other files can also be *ignored* by the `.zng-template/ignore`
file. The ignore file uses the same syntax as `.gitignore`, the paths are relative to the workspace root.

### Post

If `.zng-template/post` is present it is executed after the template replacements are applied.

If `post/post.sh` exists it is executed as a Bash script. Tries to run in $ZR_SH, $PROGRAMFILES/Git/bin/bash.exe, bash, sh.

If `post/Cargo.toml` exists it is executed as a cargo binary. The crate is build with the dev/debug profile quietly.

The post script or crate runs at the workspace root, if the exit code is not 0 `cargo new` fails. 

Note that template keys are replaced on the `post/**` files too, so code can be configured by template keys. The `.zng-template` directory
is ignored, so the post folder will not be present in current dir, rather it will be in the `ZNG_TEMPLATE_POST_DIR` environment variable.

## `l10n`

Localization text scraper.

<!--do doc --readme do zng l10n --help -->
```console
$ cargo zng l10n --help

```

Also see [`zng::l10n::l10n!`] docs for more details about the expected format.

[`zng::l10n::l10n!`]: https://zng-ui.github.io/doc/zng/l10n/macro.l10n.html#scrap-template

## `res`

Build resources

<!--do doc --readme do zng res --help -->
```console
$ cargo zng res --help

```

This subcommand can be used to build resources and package releases. It is very simple, you create
a resources directory tree as close as possible to the final resources structure, and place special
`.zr-{tool}` files on it that are calls to `cargo-zng-res-{tool}` crates or executables.

### Resource Build

The resource build follows these steps:

* The TARGET dir is wiped clean.
* The SOURCE dir is walked, matching directories are crated on TARGET, `.zr-*` tool requests are run.
* The TARGET dir is walked, any new `.zr-*` request generated by previous pass are run (request is removed after tool run).
  - This repeats until a pass does not find any `.zr-*` or the `--recursion_limit` is reached.
* Run all tools that requested `zng-res::on-final=` from a request that still exists.

### Tools

You can call `cargo zng res --tools` to see help for all tools available. Tools are searched in this order:

* If a crate exists in `tools/cargo-zng-res-{tool}` executes it (with `--quiet` build).
* If a crate exists in `tools/cargo-zng-res` and it has a `src/bin/{tool}.rs` file executes it with `--bin {tool}`.
* If the tool is builtin, executes it.
* If a `cargo-zng-res-{tool}[.exe]` is installed in the same directory as the running `cargo-zng[.exe]`, executes it.

#### Authoring Tools

Tools are configured using environment variables:

* `ZR_SOURCE_DIR` — Resources directory that is being build.
* `ZR_TARGET_DIR` — Target directory where resources are being built to.
* `ZR_CACHE_DIR` — Dir to use for intermediary data for the specific request. Keyed on the source dir, target dir, request file and request file content.
* `ZR_WORKSPACE_DIR` — Cargo workspace that contains the source dir. This is also the working dir (`current_dir`) set for the tool.
* `ZR_REQUEST` — Request file that called the tool.
* `ZR_REQUEST_DD` — Parent dir of the request file.
* `ZR_TARGET` — Target file implied by the request file name. That is, the request filename without `.zr-{tool}` and in the equivalent target subdirectory.
* `ZR_TARGET_DD` — Parent dir of thr target file.
* `ZR_FINAL` — Set to the args if the tool requested `zng-res::on-final={args}`.
* `ZR_HELP` — Print help text for `cargo zng res --tools`. If this is set the other vars will not be set.

In a Cargo workspace the [`zng::env::about`] metadata is also extracted from the primary binary crate:

* `ZR_APP` — package.metadata.zng.about.app or package.name
* `ZR_ORG` — package.metadata.zng.about.org or the first package.authors
* `ZR_VERSION` — package.version
* `ZR_DESCRIPTION` — package.description
* `ZR_HOMEPAGE` — package.homepage
* `ZR_LICENSE` — package.license
* `ZR_PKG_NAME` — package.name
* `ZR_PKG_AUTHORS` — package.authors
* `ZR_CRATE_NAME` — package.name in snake_case
* `ZR_QUALIFIER` — package.metadata.zng.about.qualifier

[`zng::env::about`]: https://zng-ui.github.io/doc/zng_env/struct.About.html

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

### Builtin Tools

These are the builtin tools provided:

<!--do doc --readme do zng res --tools -->
```console
$ cargo zng res --tools

```

The expanded help for each:

#### `.zr-copy`

<!--do doc --readme do zng res --tool copy -->
```console
$ cargo zng res --tool copy

```

#### `.zr-glob`

<!--do doc --readme do zng res --tool glob -->
```console
$ cargo zng res --tool glob

```

#### `.zr-rp`

<!--do doc --readme do zng res --tool rp -->
```console
$ cargo zng res --tool rp

```

#### `.zr-sh`

<!--do doc --readme do zng res --tool sh -->
```console
$ cargo zng res --tool sh

```

#### `.zr-shf`

<!--do doc --readme do zng res --tool shf -->
```console
$ cargo zng res --tool shf

```

#### `.zr-warn`

<!--do doc --readme do zng res --tool warn -->
```console
$ cargo zng res --tool warn

```

#### `.zr-fail`

<!--do doc --readme do zng res --tool fail -->
```console
$ cargo zng res --tool fail

```
