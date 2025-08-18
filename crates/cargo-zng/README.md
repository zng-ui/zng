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

Zng project manager.

Usage: cargo zng <COMMAND>

Commands:
  fmt    Format code and macros
  new    New project from a Zng template repository
  l10n   Localization text scraper
  res    Build resources
  trace  Run an app with trace recording enabled
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## `fmt`

Formats the code with `rustfmt` and formats Zng macros and some other braced macros.

<!--do doc --readme do zng fmt --help -->
```console
$ cargo zng fmt --help

Format code and macros

Runs cargo fmt and formats Zng macros

Usage: cargo zng fmt [OPTIONS]

Options:
      --check
          Only check if files are formatted

      --manifest-path <MANIFEST_PATH>
          Format the crate identified by Cargo.toml

  -p, --package <PACKAGE>
          Format the workspace crate identified by package name

  -f, --files <FILES>
          Format all .rs and .md files matched by glob

  -s, --stdin
          Format the stdin to the stdout

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

The formatter supports Zng macros and also attempts to format all braced macro contents 
like `foo! { <contents> }` by copying it into a temporary item `fn _fmt_item() { <contents> }` 
and trying `rustfmt`, if the contents cannot be formatted like this they are not touched.

The formatter also attempts to format Rust markdown code blocks and doctest code blocks. To skip formatting
a Rust code block or doctest use the attributes `rust,no_fmt`, for doctests the attributes `ignore` and `compile_fail`
also skip formatting.

When called for the workspace or with a `--manifest-path` the formatter will format any `**/*.rs` and `**/*.md` file
in each crate folder, except for those in the target directory. For workspaces it will also format the `./README.md` file
and `./docs/**/*.md` files.

The formatter will **not consider crate edition**, it always uses the `--edition` value.

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

New project from a Zng template repository

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

          Can be a .git URL or an `owner/repo` for a GitHub repository. Can also be an absolute path or `./path` to a local template directory.

          Use `#branch` to select a branch, that is `owner/repo#branch`.

          [default: zng-ui/zng-template]

  -s, --set [<SET>...]
          Set a template value

          Templates have a `.zng-template/keys` file that defines the possible options.

          EXAMPLE

          -s"key=value" -s"k2=v2"

  -k, --keys
          Show all possible values that can be set on the template

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
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

Localization text scraper

See the docs for `l10n!` for more details about the expected format.

Usage: cargo zng l10n [OPTIONS]

Options:
  -i, --input <INPUT>
          Rust files glob or directory

          [default: ]

  -o, --output <OUTPUT>
          L10n resources dir

          [default: ]

  -p, --package <PACKAGE>
          Package to scrap and copy dependencies

          If set the --input and --output default is src/**.rs and l10n/

          [default: ]

      --manifest-path <MANIFEST_PATH>
          Path to Cargo.toml of crate to scrap and copy dependencies

          If set the --input and --output default to src/**.rs and l10n/

          [default: ]

      --no-deps
          Don't copy dependencies localization

          Use with --package or --manifest-path to not copy {dep-pkg}/l10n/*.ftl files

      --no-local
          Don't scrap `#.#.#-local` dependencies

          Use with --package or --manifest-path to not scrap local dependencies.

      --no-pkg
          Don't scrap the target package.

          Use with --package or --manifest-path to only scrap dependencies.

      --clean-deps
          Remove all previously copied dependency localization files

      --clean-template
          Remove all previously scraped resources before scraping

      --clean
          Same as --clean-deps --clean-template

  -m, --macros <MACROS>
          Custom l10n macro names, comma separated

          [default: ]

      --pseudo <PSEUDO>
          Generate pseudo locale from dir/lang

          EXAMPLE

          "l10n/en" generates pseudo from "l10n/en.ftl" and "l10n/en/*.ftl"

          [default: ]

      --pseudo-m <PSEUDO_M>
          Generate pseudo mirrored locale

          [default: ]

      --pseudo-w <PSEUDO_W>
          Generate pseudo wide locale

          [default: ]

      --check
          Only verify that the generated files are the same

  -v, --verbose
          Use verbose output

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Also see [`zng::l10n::l10n!`] docs for more details about the expected format.

[`zng::l10n::l10n!`]: https://zng-ui.github.io/doc/zng/l10n/macro.l10n.html#scrap-template

## `res`

Build resources

<!--do doc --readme do zng res --help -->
```console
$ cargo zng res --help

Build resources

Builds resources SOURCE to TARGET, delegates `.zr-{tool}` files to `cargo-zng-res-{tool}` executables and crates.

Usage: cargo zng res [OPTIONS] [SOURCE] [TARGET]

Arguments:
  [SOURCE]
          Resources source dir

          [default: res]

  [TARGET]
          Resources target dir

          This directory is wiped before each build.

          [default: target/res]

Options:
      --pack
          Copy all static files to the target dir

      --tool-dir <DIR>
          Search for `zng-res-{tool}` in this directory first

          [default: tools]

      --tools
          Prints help for all tools available

      --tool <TOOL>
          Prints the full help for a tool

      --tool-cache <TOOL_CACHE>
          Tools cache dir

          [default: target/res.cache]

      --recursion-limit <RECURSION_LIMIT>
          Number of build passes allowed before final

          [default: 32]

      --metadata <TOML_FILE>
          TOML file that that defines metadata uses by tools (ZR_APP, ZR_ORG, ..)

          This is only needed if the workspace has multiple bin crates and none or many set '[package.metadata.zng.about]'.

          See `zng::env::About` for more details.

      --metadata-dump
          Writes the metadata extracted the workspace or --metadata

  -v, --verbose
          Use verbose output

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
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

.zr-copy @ cargo-zng
  Copy the file or dir

.zr-glob @ cargo-zng
  Copy all matches in place

.zr-rp @ cargo-zng
  Replace ${VAR|<file|!cmd} occurrences in the content

.zr-sh @ cargo-zng
  Run a bash script

.zr-shf @ cargo-zng
  Run a bash script on the final pass

.zr-warn @ cargo-zng
  Print a warning message

.zr-fail @ cargo-zng
  Print an error message and fail the build

.zr-apk @ cargo-zng
  Build an Android APK from a staging directory

call 'cargo zng res --help tool' to read full help from a tool
```

The expanded help for each:

#### `.zr-copy`

<!--do doc --readme do zng res --tool copy -->
```console
$ cargo zng res --tool copy

.zr-copy</bold> @ cargo-zng
  Copy the file or dir

  The request file:
    source/foo.txt.zr-copy
     | # comment
     | path/bar.txt

  Copies `path/bar.txt` to:
    target/foo.txt

  Paths are relative to the Cargo workspace root.
```

#### `.zr-glob`

<!--do doc --readme do zng res --tool glob -->
```console
$ cargo zng res --tool glob

.zr-glob</bold> @ cargo-zng
  Copy all matches in place

  The request file:
    source/l10n/fluent-files.zr-glob
     | # localization dir
     | l10n
     | # only Fluent files
     | **/*.ftl
     | # except test locales
     | !:**/pseudo*

  Copies all '.ftl' not in a *pseudo* path to:
    target/l10n/

  The first path pattern is required and defines the entries that
  will be copied, an initial pattern with '**' flattens the matches.
  The path is relative to the Cargo workspace root.

  The subsequent patterns are optional and filter each file or dir selected by
  the first pattern. The paths are relative to each match, if it is a file
  the filters apply to the file name only, if it is a dir the filters apply to
  the dir and descendants.

  The glob pattern syntax is:

      ? — matches any single character.
      * — matches any (possibly empty) sequence of characters.
     ** — matches the current directory and arbitrary subdirectories.
    [c] — matches any character inside the brackets.
  [a-z] — matches any characters in the Unicode sequence.
   [!b] — negates the brackets match.

  And in filter patterns only:

  !:pattern — negates the entire pattern.
```

#### `.zr-rp`

<!--do doc --readme do zng res --tool rp -->
```console
$ cargo zng res --tool rp

.zr-rp</bold> @ cargo-zng
  Replace ${VAR|<file|!cmd} occurrences in the content

  The request file:
    source/greetings.txt.zr-rp
     | Thanks for using ${ZR_APP}!

  Writes the text content with ZR_APP replaced:
    target/greetings.txt
    | Thanks for using Foo App!

  The parameters syntax is ${VAR|!|<[:[case]][?else]}:

  ${VAR}          — Replaces with the env var value, or fails if it is not set.
  ${VAR:case}     — Replaces with the env var value, case converted.
  ${VAR:?else}    — If VAR is not set or is empty uses 'else' instead.

  ${<file.txt}    — Replaces with the 'file.txt' content.
                    Paths are relative to the workspace root.
  ${<file:case}   — Replaces with the 'file.txt' content, case converted.
  ${<file:?else}  — If file cannot be read or is empty uses 'else' instead.

  ${!cmd -h}      — Replaces with the stdout of the bash script line.
                    The script runs the same bash used by '.zr-sh'.
                    The script must be defined all in one line.
                    A separate bash instance is used for each occurrence.
                    The working directory is the workspace root.
  ${!cmd:case}    — Replaces with the stdout, case converted.
                    If the script contains ':' quote it with double quotes"
  $!{!cmd:?else}  — If script fails or ha no stdout, uses 'else' instead.

  $${VAR}         — Escapes $, replaces with '${VAR}'.

  The :case functions are:

  :k or :kebab  — kebab-case (cleaned)
  :K or :KEBAB  — UPPER-KEBAB-CASE (cleaned)
  :s or :snake  — snake_case (cleaned)
  :S or :SNAKE  — UPPER_SNAKE_CASE (cleaned)
  :l or :lower  — lower case
  :U or :UPPER  — UPPER CASE
  :T or :Title  — Title Case
  :c or :camel  — camelCase (cleaned)
  :P or :Pascal — PascalCase (cleaned)
  :Tr or :Train — Train-Case (cleaned)
  :           — Unchanged
  :clean      — Cleaned
  :f or :file — Sanitize file name

  Cleaned values only keep ascii alphabetic first char and ascii alphanumerics, ' ', '-' and '_' other chars.
  More then one case function can be used, separated by pipe ':T|f' converts to title case and sanitize for file name.


  The fallback(:?else) can have nested ${...} patterns.
  You can set both case and else: '${VAR:case?else}'.

  Variables:

  All env variables can be used, of particular use with this tool are:

  ZR_APP — package.metadata.zng.about.app or package.name
  ZR_ORG — package.metadata.zng.about.org or the first package.authors
  ZR_VERSION — package.version
  ZR_DESCRIPTION — package.description
  ZR_HOMEPAGE — package.homepage
  ZR_LICENSE — package.license
  ZR_PKG_NAME — package.name
  ZR_PKG_AUTHORS — package.authors
  ZR_CRATE_NAME — package.name in snake_case
  ZR_QUALIFIER — package.metadata.zng.about.qualifier

  See `zng::env::about` for more details about metadata vars.
  See the cargo-zng crate docs for a full list of ZR vars.
```

#### `.zr-sh`

<!--do doc --readme do zng res --tool sh -->
```console
$ cargo zng res --tool sh

.zr-sh</bold> @ cargo-zng
  Run a bash script

  Script is configured using environment variables (like other tools):

  ZR_SOURCE_DIR — Resources directory that is being build.
  ZR_TARGET_DIR — Target directory where resources are being built to.
  ZR_CACHE_DIR — Dir to use for intermediary data for the specific request.
  ZR_WORKSPACE_DIR — Cargo workspace that contains source dir. Also the working dir.
  ZR_REQUEST — Request file that called the tool (.zr-sh).
  ZR_REQUEST_DD — Parent dir of the request file.
  ZR_TARGET — Target file implied by the request file name.
  ZR_TARGET_DD — Parent dir of the target file.

  ZR_FINAL — Set if the script previously printed `zng-res::on-final={args}`.

  In a Cargo workspace the `zng::env::about` metadata is also set:

  ZR_APP — package.metadata.zng.about.app or package.name
  ZR_ORG — package.metadata.zng.about.org or the first package.authors
  ZR_VERSION — package.version
  ZR_DESCRIPTION — package.description
  ZR_HOMEPAGE — package.homepage
  ZR_LICENSE — package.license
  ZR_PKG_NAME — package.name
  ZR_PKG_AUTHORS — package.authors
  ZR_CRATE_NAME — package.name in snake_case
  ZR_QUALIFIER — package.metadata.zng.about.qualifier

  Script can make requests to the resource builder by printing to stdout.
  Current supported requests:

  zng-res::warning={msg} — Prints the `{msg}` as a warning after the script exits.
  zng-res::on-final={args} — Schedule second run with `ZR_FINAL={args}`, on final pass.

  If the script fails the entire stderr is printed and the resource build fails. Scripts run with
  `set -e` by default.

  Tries to run on $ZR_SH, $PROGRAMFILES/Git/bin/bash.exe, bash, sh.
```

#### `.zr-shf`

<!--do doc --readme do zng res --tool shf -->
```console
$ cargo zng res --tool shf

.zr-shf</bold> @ cargo-zng
  Run a bash script on the final pass

  Apart from running on final this tool behaves exactly like .zr-sh
```

#### `.zr-warn`

<!--do doc --readme do zng res --tool warn -->
```console
$ cargo zng res --tool warn

.zr-warn</bold> @ cargo-zng
  Print a warning message

  You can combine this with '.zr-rp' tool

  The request file:
    source/warn.zr-warn.zr-rp
     | ${ZR_APP}!

  Prints a warning with the value of ZR_APP
```

#### `.zr-fail`

<!--do doc --readme do zng res --tool fail -->
```console
$ cargo zng res --tool fail

.zr-fail</bold> @ cargo-zng
  Print an error message and fail the build

  The request file:
    some/dir/disallow.zr-fail.zr-rp
     | Don't copy ${ZR_REQUEST_DD} with a glob!

  Prints an error message and fails the build if copied
```
