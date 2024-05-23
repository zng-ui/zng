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
