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

Usage: cargo-zng.exe new [OPTIONS] <NAME>

Arguments:
  <NAME>
          Project Name

          Can be a simple "name" or a "qualifier/org/project-name".

          EXAMPLES

          "br.com/My Org/My App" generates a `./my-app` project and sets metadata

          "my_app" generates a `./my_app` project

          "My App" generates a `./my-app` project

Options:
  -t, --template <TEMPLATE>
          Zng template

          Can be `.git` URL or an `owner/repo` for a GitHub repository.

          [default: zng-ui/zng-template]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Zng templates are very simple, it does not use any template engine, just Rust's string replace in UTF-8 text files only.
The replacement keys are designed to be compilable, so template designers can build/check their template like a normal Rust project.

These are current supported keys, for an example name `"rs.qualifier/My Org/My App"`:

##### In file/folder names and text

* `t-app-t` — my-app
* `t_app_t` — my_app
* `T_APP_T` — MY_APP
* `T-APP-T` — MY-APP

##### Only in text

* `t.App.t` — My App
* `t-App-t` — My-App
* `t.Org.t` — My Org
* `t-Org-t` — My-Org
* `t.qualifier.t` — rs.qualifier

See [zng-ui/zng-template] for an example of templates.

[zng-ui/zng-template]: https://github.com/zng-ui/zng-template
