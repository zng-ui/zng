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

The code examples are for Ubuntu, but it should be similar for other systems.

```console
curl -L -o translateLocally.deb "https://github.com/XapaJIaMnu/translateLocally/releases/download/latest/translateLocally-v0.0.2+8e31cff-Ubuntu-22.04.AVX.deb"
sudo apt-get install ./translateLocally.deb
rm translateLocally.deb
```

### Translation Models

Some models are available to be installed by `translateLocally`.

```console
translateLocally --available-models
```

Other models are provided by [Mozilla](https://mozilla.github.io/translations/model-registry/) and can be 
[installed manually](https://github.com/XapaJIaMnu/translateLocally?tab=readme-ov-file#importing-custom-models).

For this example lets install the English to Spanish model:

```console
translateLocally -d en-es-tiny
```

And configure the service:

```console
export CARGO_ZNG_TRANSLATE='translateLocally -m "{from}-{to}-tiny" --input "{text}"'
```

### Translating

With the enviroment configured we can translate:

```console
cargo do zng l10n --translate-from en --translate-to es --translate crates/zng/l10n/template
```
