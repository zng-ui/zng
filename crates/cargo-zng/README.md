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

Usage: cargo zng [OPTIONS] <COMMAND>

Commands:
  fmt    Format code and macros
  new    New project from a Zng template repository
  l10n   Localization text scraper
  res    Build resources
  trace  Run an app with trace recording enabled
  help   Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Use verbose output (-vv very verbose)
  -h, --help        Print help
  -V, --version     Print version
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
          Format all files matched by glob

  -s, --stdin
          Format the stdin to the stdout

      --edition <EDITION>
          Rustfmt style edition, enforced for all files

          [default: 2024]

  -v, --verbose...
          Use verbose output (-vv very verbose)

      --full
          Format or check every file, not just changed files

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

  -v, --verbose...
          Use verbose output (-vv very verbose)

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
  -i, --input <PATH>
          Rust files glob or directory

  -o, --output <DIR>
          L10n resources dir

  -p, --package <PACKAGE>
          Package to scrap and copy dependencies

          If set the --input and --output default is src/**.rs and l10n/

      --manifest-path <MANIFEST_PATH>
          Path to Cargo.toml of crate to scrap and copy dependencies

          If set the --input and --output default to src/**.rs and l10n/

      --no-deps
          Don't copy dependencies localization

          Use with --package or --manifest-path to not copy {dep-pkg}/l10n/*.ftl files

      --no-local
          Don't scrap `#.#.#-local` dependencies

          Use with --package or --manifest-path to not scrap local dependencies.

  -v, --verbose...
          Use verbose output (-vv very verbose)

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

      --pseudo <PATH>
          Generate pseudo locale from dir/lang

          EXAMPLE

          "l10n/en" generates pseudo from "l10n/en/**/*.ftl" to "l10n/pseudo"

      --pseudo-m <PATH>
          Generate pseudo mirrored locale

      --pseudo-w <PATH>
          Generate pseudo wide locale

      --release-langs <PATH>
          Output comma separated list of langs that would be included by 'l10n sourcing localization from the given PATH

          See cargo zng res --tool l10n for details

      --translate <PATH>
          Machine translate locale from dir/lang

          EXAMPLE

          "l10n/template" translates from "l10n/template/**/*.ftl" to a folder for each --translate-to language

      --translate-from <LANG>
          Explicit source language for --translate

          By default is the source folder name, or English for `template`

      --translate-to <LANGS>
          Target languages for --translate

          [default: ar,bg,ca,cs,da,de,el,en,es-419,es-ES,et,eu,fi,fr-CA,fr-FR,gl,he,hi,hr,hu,id,it,ja,ko,lt,lv,nb,nl,pl,pt-BR,pt-PT,ro,ru,sk,sl,sr-Latn,sv,th,tr,uk,vi,zh-Hans,zh-Hant]

      --translate-replace
          Replace all existing machine translations with --translate

          By default only replaces stale translations

      --check
          Verify that packages are scrapped and validate Fluent files

      --check-strict
          Require that all template keys be present in all localized files

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

Builds resources SOURCE to TARGET, delegates `'{tool}` files to `cargo-zng-res-{tool}` executables and crates.

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

  -v, --verbose...
          Use verbose output (-vv very verbose)

      --metadata <TOML_FILE>
          TOML file that that defines metadata uses by tools (ZR_APP, ZR_ORG, ..)

          This is only needed if the workspace has multiple bin crates and none or many set '[package.metadata.zng.about]'.

          See `zng::env::About` for more details.

      --metadata-dump
          Writes the metadata extracted the workspace or --metadata

      --upgrade-zr <DIR>
          Rename all "name.zr-tool" files to new syntax "name 'tool"

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

This subcommand can be used to build resources and package releases. It is very simple, you create
a resources directory tree as close as possible to the final resources structure, and place special
`name '{tool}` files on it that are calls to `cargo-zng-res-{tool}` crates or executables.

### Resource Build

The resource build follows these steps:

* The TARGET dir is wiped clean.
* The SOURCE dir is walked, matching directories are crated on TARGET
  - Normal files are copied to TARGET.
  - `* 'tool` files run the tool, the output is written to TARGET.
    - Only the first tool runs (that is `file 'a'b`) will run `'a` and output to `TARGET/file 'b`
    - After each tool run the TARGET dir is walked, any nested or new `'tool` is called.
    - This can repeat until there are no tools in TARGET or the `--recursion-limit` is reached.
  - Files are processed in order (natural sort).
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
* `ZR_TARGET_DD` — Parent dir of the target file.
* `ZR_FINAL` — Set to the args if the tool requested `zng-res::on-final={args}`.
* `ZR_HELP` — Print help text for `cargo zng res --tools`. If this is set the other vars will not be set.
* `ZR_VERBOSE` — Print extra output. No value means not verbose, any value indicates verbose, currently `"v"` is used.

In a Cargo workspace the [`zng::env::about`] metadata is also extracted from the primary binary crate:

* `ZR_APP_ID` — package.metadata.zng.about.app_id or "qualifier.org.app" in snake_case
* `ZR_APP` — package.metadata.zng.about.app or package.name
* `ZR_ORG` — package.metadata.zng.about.org or the first package.authors
* `ZR_VERSION` — package.version
* `ZR_DESCRIPTION` — package.description
* `ZR_HOMEPAGE` — package.homepage
* `ZR_LICENSE` — package.license
* `ZR_PKG_NAME` — package.name
* `ZR_PKG_AUTHORS` — package.authors
* `ZR_CRATE_NAME` — package.name in snake_case
* `ZR_QUALIFIER` — package.metadata.zng.about.qualifier or the first components `ZR_APP_ID` except the last two
* `ZR_META_*` — any other custom string value in package.metadata.zng.about.*

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

'apk @ cargo-zng
  Build an Android APK from a staging directory

'copy @ cargo-zng
  Copy the file or dir

'fail @ cargo-zng
  Print an error message and fail the build

'glob @ cargo-zng
  Copy all matches in place

'l10n @ cargo-zng
  Copy localization files (.ftl) and optimize for release

'rp @ cargo-zng
  Replace ${VAR|<file|!cmd} occurrences in the content

'sfx @ cargo-zng
  Compile a self-extracting executable

'sfxf @ cargo-zng
  Build a self-extracting executable on the final pass

'sh @ cargo-zng
  Run a bash script

'shf @ cargo-zng
  Run a bash script on the final pass

'tar @ cargo-zng
  Pack files and dirs into a TAR container with optional compression

'warn @ cargo-zng
  Print a warning message

call 'cargo zng res --help tool' to read full help from a tool
```

The expanded help for each:

#### `'apk`

<!--do doc --readme do zng res --tool apk -->
```console
$ cargo zng res --tool apk

'apk @ cargo-zng
  Build an Android APK from a staging directory

  The expected file system layout:

  | apk/
  | ├── lib/
  | |   └── arm64-v8a
  | |       └── my-app.so
  | ├── assets/
  | |   └── res
  | |       └── zng-res.txt
  | ├── res/
  | |   └── android-res
  | └── AndroidManifest.xml
  | my-app 'apk

  Both 'apk/' and 'my-app 'apk' will be replaced with the built my-app.apk

  Expected 'apk file content:

  | # Relative path to the staging directory. If not set uses ./apk if it exists
  | # or the parent dir .. if it is named something.apk
  | apk-dir = ./apk
  |
  | # Sign using the debug key. Note that if ZR_APK_KEYSTORE or ZR_APK_KEY_ALIAS are not
  | # set the APK is also signed using the debug key.
  | debug = true
  |
  | # Don't sign and don't zipalign the APK. This outputs an incomplete package that
  | # cannot be installed, but can be modified such as custom linking and signing.
  | raw = true
  |
  | # Don't tar assets. By default `assets/res` are packed as `assets/res.tar`
  | # for use with `android_install_res`.
  | tar-assets-res = false

  APK signing is configured using these environment variables:

  ZR_APK_KEYSTORE - path to the private .keystore file
  ZR_APK_KEYSTORE_PASS - keystore file password
  ZR_APK_KEY_ALIAS - key name in the keystore
  ZR_APK_KEY_PASS - key password
```

#### `'copy`

<!--do doc --readme do zng res --tool copy -->
```console
$ cargo zng res --tool copy

'copy @ cargo-zng
  Copy the file or dir

  The request file:
    source/foo.txt 'copy
     | # comment
     | path/bar.txt

  Copies `path/bar.txt` to:
    target/foo.txt

  Paths are relative to the Cargo workspace root
```

#### `'fail`

<!--do doc --readme do zng res --tool fail -->
```console
$ cargo zng res --tool fail

'fail @ cargo-zng
  Print an error message and fail the build

  The request file:
    some/dir/disallow 'rp'fail
     | Don't copy ${ZR_REQUEST_DD} with a glob!

  Prints an error message and fails the build if copied
```


#### `'glob`

<!--do doc --readme do zng res --tool glob -->
```console
$ cargo zng res --tool glob

'glob @ cargo-zng
  Copy all matches in place

  The request file:
    source/l10n/fluent-files 'glob
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

       ? — Matches any single character.
       * — Matches any (possibly empty) sequence of characters.
      ** — Matches the current directory and arbitrary subdirectories.
     [c] — Matches any character inside the brackets.
    [!c] — Negates the brackets match.
   {a,b} — Matches any of the inner patterns.

  And in filter patterns only:

  !:pattern — negates the entire pattern.

  Matching is case insensitive on all platforms.
```

#### `'l10n`

<!--do doc --readme do zng res --tool l10n -->
```console
$ cargo zng res --tool l10n

'l10n @ cargo-zng
  Copy localization files (.ftl) and optimize for release

  The request file:
    source/l10n 'l10n
     | # comment
     | path/dev-l10n

  Copies the `path/dev-l10n` dir to:
    target/l10n

  Paths are relative to the Cargo workspace root

  Filter:

  Only localization files are included
      **/*.ftl

  Development langs are excluded
      !./pseudo*
      !./template

  Only lang folders that have local translations are included
      If ./{lang}/deps/** exists but no ./{lang}/*.ftl exists it is excluded

  Comments are stripped

  Subsetting:

  If a l10n subset profile is found is is applied to the dependency localization

  The subset profile is an allow list, see the docs `zng::l10n` for how to create one

  The subset profile is resolved in this order:

  ZNG_L10N_PROFILE_FILE env if is set
      Must be set to a .subset file path, relative to the Cargo workspace root
      If the file is a {name}.rec.subset auto includes a {name}.subset and vice versa

  res/optimization-profiles/zng-ext-l10n.rec.subset
      Default location, also includes zng-ext-l10n.subset if present

  {l10n-path}/*.subset
      If multiple files match all are used
```

#### `'rp`

<!--do doc --readme do zng res --tool rp -->
```console
$ cargo zng res --tool rp

'rp @ cargo-zng
  Replace ${VAR|<file|!cmd} occurrences in the content

  The request file:
    source/greetings.txt 'rp
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
                    The script runs the same bash used by 'sh.
                    The script must be defined all in one line.
                    A separate bash instance is used for each occurrence.
                    The working directory is the workspace root.
  ${!cmd:case}    — Replaces with the stdout, case converted.
                    If the script contains ':' quote it with double quotes\"
  ${!cmd:?else}  — If script fails or ha no stdout, uses 'else' instead.

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

  ZR_APP_ID — package.metadata.zng.about.app_id or "qualifier.org.app" in snake_case
  ZR_APP — package.metadata.zng.about.app or package.name
  ZR_ORG — package.metadata.zng.about.org or the first package.authors
  ZR_VERSION — package.version
  ZR_DESCRIPTION — package.description
  ZR_HOMEPAGE — package.homepage
  ZR_LICENSE — package.license
  ZR_PKG_NAME — package.name
  ZR_PKG_AUTHORS — package.authors
  ZR_CRATE_NAME — package.name in snake_case
  ZR_QUALIFIER — package.metadata.zng.about.qualifier or the first components `ZR_APP_ID` except the last two
  ZR_META_*` — any other custom string value in package.metadata.zng.about.*

  See `zng::env::about` for more details about metadata vars.
  See the cargo-zng crate docs for a full list of ZR vars.
```

#### `'sfx`

<!--do doc --readme do zng res --tool sfx -->
```console
$ cargo zng res --tool sfx

'sfx @ cargo-zng
  Compile a self-extracting executable

  The request file:
    source/sfx-package 'sfx
     | [sfx]
     | # executable to run, required
     | run = "target/release/run"
     |
     | # Embedded icon for the sfx executable (Windows only)
     | icon = "res/sfx.ico"
     |
     | # optional args for 'run'
     | args = ["--foo"]
     | # optional extra env for 'run'
     | env = {
     |     FOO = "bar",
     | }
     |
     | # rustc target triple, default is the host triple
     | # rustc-target = "x86_64-pc-windows-msvc"
     | # build a console exe on Windows, default is true (build a GUI exe)
     | windows-subsystem = false
     |
     | # compression to use for 'run', default is "zstd-bcj"
     | # compress = "none"
     |
     | # data the sfx can serve the 'run'
     | [[data]]
     | # name must be unique and not include ':' or '\n', default is "", for single data
     | name = "payload"
     | # compress data on build, default is "zstd"
     | compress = "zstd"
     | # file to include
     | file = "./data.tar"
     |
     | [sign]
     | # optional, code sign the sfx exe
     | tool = "signtool sign /v /f $PFX /tr http://timestamp.sectigo.com /td SHA256 /fd SHA256 $SIGN_TARGET"
     | # only sign the sfx exe, default 'false' signs the 'run' exe too
     | # sfx-only = true

  Compiles and signs a 'sfx-package.exe' with custom icon on Windows, or a 'sfx-package' on Unix.

  Run:

  When sfx runs it extracts the 'run' executable to a temp dir and runs it.

  The optional 'env' variables override the system env. The SFX_ARGS var is always set.

  The SFX_ARGS is set to the sfx command line args, '\n' separated. The first arg is the path to the sfx exe.

  On build, also searches for "$run.exe" if "$run" is not found and has no extension.

  Data:

  To read data the 'run' exe must spawn another instance of the sfx with the "SFX_GET_DATA" set
  to the entry name. It will serve the data to stdout. The data may be decompressed on demand.

  To get a list of data names and decompressed lengths run with "SFX_GET_MANIFEST", each stdout
  line is <name>:<len>, <len> is an u64 or "unknown".

  The `zng::setup::SfxClient` can also be used to connect and get data.

  File Paths:

  Paths are relative to the Cargo workspace root, you can also use 'rp to select files in the
  resource target dir.

  This request file:
    source/sfx-package 'rp'sfxf
     | [[data]]
     | file = "${ZR_TARGET_DD}/res.txt"

  Compiles a 'sfx-package' that includes the 'res.txt' copied to the target dir by `cargo zng res`.

  Compress:

  The sfx exe includes a zstd decompressor that is used to extract the 'run' exe.

  The decompressor code can be used to read data too. The 'compress' field values are:

  "none" — No compression on build. Data is served as is.
  "zstd" — Compress on build unless file is already zstd (magic number check). Decompress on demand while reading.
  "zstd-[filter]" — Transform data to improve compression, unless file is already zstd. Reverses
    transform on demand while reading.

  Sfx is optimized for small number of large data entries. Use a container format to
  package many small entries.

  Filter:

  Currently only BCJ (Branch/Call/Jump) filters are supported, identified by CPU instruction set:

  "zstd-bcj-[set]" where [set] is: "x86", "arm", "arm64", "arm-thumb", "ppc", "sparc", "ia64", "riscv".
  "zstd-bcj" — Select filter from 'rustc-target' arch, or zstd unfiltered for no matches.

  The target file must be a binary (exe or lib) or a container (like tar) with only binary entries. The filters
  are non-destructive but if the wrong filter is selected it will have negative impact on the compression level.

  Signing:

  Code signing must be applied to both the run exe and sfx exe, to facilitate this you can set the 'sign.tool'.

  The sign-tool command will run twice, with $SIGN_TARGET set to "./run.exe" and "package.exe".

  In the example above The $PFX var is an example of how to set the the private key.
  Keep the private key file outside the repository and set an env var to it. In CI use
  secure variables.

  Icon:

  On Windows the sfx executable icon can be set with 'icon' field. Note that this requires the build
  to run on a Windows machine with MSVC Toolkit installed. Cross-compilation from other systems will not work.
```

#### `'sfxf`

<!--do doc --readme do zng res --tool sfxf -->
```console
$ cargo zng res --tool sfxf

'sfxf @ cargo-zng
  Build a self-extracting executable on the final pass

  Apart from running on final this tool behaves exactly like 'sfx
```

#### `'sh`

<!--do doc --readme do zng res --tool sh -->
```console
$ cargo zng res --tool sh

'sh @ cargo-zng
  Run a bash script

  Script is configured using environment variables (like other tools):

  ZR_SOURCE_DIR — Resources directory that is being build.
  ZR_TARGET_DIR — Target directory where resources are being built to.
  ZR_CACHE_DIR — Dir to use for intermediary data for the specific request.
  ZR_WORKSPACE_DIR — Cargo workspace that contains source dir. This is also the working dir.
  ZR_REQUEST — Request file that called the tool ('sh).
  ZR_REQUEST_DD — Parent dir of the request file.
  ZR_TARGET — Target file implied by the request file name.
  ZR_TARGET_DD — Parent dir of the target file.

  ZR_FINAL — Set if the script previously printed `zng-res::on-final={args}`.

  In a Cargo workspace the `zng::env::about` metadata is also set:

  ZR_APP_ID — package.metadata.zng.about.app_id or "qualifier.org.app" in snake_case
  ZR_APP — package.metadata.zng.about.app or package.name
  ZR_ORG — package.metadata.zng.about.org or the first package.authors
  ZR_VERSION — package.version
  ZR_DESCRIPTION — package.description
  ZR_HOMEPAGE — package.homepage
  ZR_LICENSE — package.license
  ZR_PKG_NAME — package.name
  ZR_PKG_AUTHORS — package.authors
  ZR_CRATE_NAME — package.name in snake_case
  ZR_QUALIFIER — package.metadata.zng.about.qualifier or the first components `ZR_APP_ID` except the last two
  ZR_META_* — any other custom string value in package.metadata.zng.about.*

  Script can make requests to the resource builder by printing to stdout.
  Current supported requests:

  zng-res::warning={msg} — Prints the `{msg}` as a warning after the script exits.
  zng-res::on-final={args} — Schedule second run with `ZR_FINAL={args}`, on final pass.

  If the script fails the entire stderr is printed and the resource build fails. Scripts run with
  `set -e` by default.

  Tries to run on $ZR_SH, $PROGRAMFILES/Git/bin/bash.exe, bash, sh.
```

#### `'shf`

<!--do doc --readme do zng res --tool shf -->
```console
$ cargo zng res --tool shf

'shf @ cargo-zng
  Run a bash script on the final pass

  Apart from running on final this tool behaves exactly like 'sh
```

#### `'tar`

<!--do doc --readme do zng res --tool tar -->
```console
$ cargo zng res --tool tar

'tar @ cargo-zng
  Pack files and dirs into a TAR container with optional compression

  The request file:
    source/data.tar.zst 'tar
     | [[entry]]
     | path = "res/bin/*"
     | name = "bin/*"
     |
     | [[entry]]
     | path = "README.md"
     | name = "docs/README.md"
     |
     | # Optional compression
     | [zstd]
     | level = 19

  Packs entries into a TAR, compresses it with ZSTD.

  The syntax is a TOML file, the tables are:

  [[entry]] — Array of entries to pack.
  path — Path or glob pattern, relative to the workspace root. Required.
  name — Optional name of the file on the TAR container. Optional.

  Each entry can be a file, directory or glob selection. Only file and directory
  entries are supported by this tool, other tar archive entries are not supported.

  If 'path' matches a directory all contained files and sub directories are packed.

  Matching is case insensitive in all platforms. See 'glob tool for glob syntax.

  If 'name' is not set it is path relative to workspace root.

  If 'name' is set it must not contain "/..".

  If 'name' ends with "/*" the selected files and dirs are named in this TAR dir.

  All 'name' paths are relative to the TAR root, "/foo" is packed as "foo".

  [filter] — Optional lossless transform to apply to the TAR
  bcj — Branch/Call/Jump filter that optimizes compression of binary code files.
      Values: "x86", "arm", "arm64", "arm-thumb", "ppc", "sparc", "ia64", "riscv"
      Note that decompressor must revert the filter.

  The decompressor must undo these changes before reading the TAR. The 'sfx supports decoding BCJ.

  [zstd] — Optional ZStandard compression
  level — Compression level, -131072..=22, 0 means no compression, default is 19.

  Compress the TAR, after [filter] is applied, using ZStandard. The "contentsize" field of zstd header
  is correctly set, so this is fully compatible with 'sfx that uses this field to report response stream length.

  [gzip] — Optional GZip compression
  level — Compression level, 0..=9, 0 means no compression, default is 8.
```

#### `'warn`

<!--do doc --readme do zng res --tool warn -->
```console
$ cargo zng res --tool warn

'warn @ cargo-zng
  Print a warning message

  You can combine this with 'rp tool

  The request file:
    source/warn 'rp'warn
     | ${ZR_APP}!

  Prints a warning with the value of ZR_APP
```
