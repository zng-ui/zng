# L10n Translate Setup

This document is a guide for how to setup a local environment to run `cargo zng l10n --translate`.

## Environment Variable

The `cargo-zng` crate only implements the parsing and generation of Fluent files, the actual translation
is done by a third party service that must be configured using the `CARGO_ZNG_TRANSLATE`.

```console
CARGO_ZNG_TRANSLATE=my-l10n --model "{from}-{to}" --input "{text}" --context "{comments}"
```

The value is a command template with substitutions:

* `{from}` - The input locale (required).
* `{to}` - The output locale (required).
* `{text}` - The text to translate (required).
* `{comments}` - The Fluent comments about the text.

The locale is in lowercase and uses underline separator, example: `pt` or `pt_br`.

## Local Setup

The local translator will run on the CPU and use the same models Firefox uses for page translation.

### Service Executable

The translation models use the Marian architecture developed by the Project Bergamot, for this setup
we will use [translateLocally](https://github.com/XapaJIaMnu/translateLocally).

Precompiled binaries are provided for macOS, Ubuntu and Windows, or you can compile it.

```console

```

2 - https://mozilla.github.io/translations/model-registry/