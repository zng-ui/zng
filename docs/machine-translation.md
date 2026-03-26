# Machine Translations

This document is a guide for how to configure and generate machine translations of localization directories using `cargo-zng`.

The `cargo-zng` crate only manages the collection of Fluent files and can detect stale translations, but it does not provide
translations by it self, for that you must install a translator plugin.

## Translator

The translator plugin must be an executable installed beside the `cargo-zng` executable, it must have a name with prefix `zng-l10n-translator-`, 
it must receive two [`Lang`] args `--from-lang` and `--to-lang`, it must read Fluent code from *stdin* and output only the translated code to *stdout*.
The plugin is instantiated for each file.

[`Lang`]: https://zng-ui.github.io/doc/zng/l10n/struct.Lang.html

### Gemini

In this guide we will use the [`zng-l10n-translator-gemini`] that uses the Google Gemini API to translate:

[`zng-l10n-translator-gemini`]: https://crates.io/crates/zng-l10n-translator-gemini

```console
cargo install zng-l10n-translator-gemini
```

This translator requires a `GEMINI_API_KEY` environment variable:

```console
export GEMINI_API_KEY=000000__000000
```

You can generate an API key in [https://aistudio.google.com/app/api-keys].

Optionally you can also set the `GEMINI_TRANSLATOR_MODEL`:

```console
export GEMINI_TRANSLATOR_MODEL=gemini-3.1-pro-preview
```

The default model is `gemini-3.1-flash-lite-preview`.

## Command

To run the translator on a localization directory the minimal command is:

```console
cargo zng l10n --translate dir/l10n/template
```

This will generate a machine translation for each of the default `--to-lang` languages and assumes the `template` language is English.

A more complete command:

```console
cargo zng l10n --translate l10n/template --translate-from en --translate-to ja --translate-replace
```

This will translate `l10n/template` from English to Japanese and will replace the existing `l10n/ja-machine` even if it is fresh.

### Translator Disambiguation

If multiple translators are installed you must define the `ZNG_L10N_TRANSLATOR` environment variable:

```console
export ZNG_L10N_TRANSLATOR=foo
```

This will select the `zng-l10n-translator-foo` translator.

You can also use a translator not installed by Cargo by setting a path:

```console
export ZNG_L10N_TRANSLATOR="./local-translator.exe"
```
