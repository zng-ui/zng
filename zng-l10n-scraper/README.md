This crate is part of the [`zng`](https://github.com/zng-ui/zng) project.

Command-line tool that scraps l10n text from Zng apps code. See [documentation] for more details.

# Installation

```console
cargo install zng-l10n-scraper
```

# Usage

```console
# zng-l10n-scraper --help

Command-line tool that scraps l10n text from Zng apps code.

Usage: zng-l10n-scraper [OPTIONS] --input <INPUT> --output <OUTPUT>

Options:
  -i, --input <INPUT>        Rust files glob
  -o, --output <OUTPUT>      Lang dir
  -m, --macros <MACROS>      Custom macro names, comma separated [default: ]
      --pseudo <PSEUDO>      Pseudo Base name, empty to disable [default: pseudo]
      --pseudo-m <PSEUDO_M>  Pseudo Mirrored name, empty to disable [default: pseudo-mirr]
      --pseudo-w <PSEUDO_W>  Pseudo Wide name, empty to disable [default: pseudo-wide]    
  -h, --help                 Print help
  -V, --version              Print version

```

[documentation]: https://zng-ui.github.io/doc/zng/l10n/macro.l10n.html#scrap-template