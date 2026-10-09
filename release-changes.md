
This release contains minor breaking changes on the API that are trivial to fix.

* **Breaking** `zng::fs_watcher` API now uses `globset` adding support for braced alternate patterns.
* **Breaking** All `zng::config` config types are now `#[non_exhaustive]` and use `Txt` strings.

* **Breaking** `zng-l10n-translator-local` now connects to local port 9931 by default.

* Add `L10nTarData::extract` helper.
    - Surface type in `zng::l10n::L10nTarData`.

* `L10N.load_tar` now supports data in `.tar.zst` format.
* `zng_tp_licenses` now uses `zstd` for compression.

* Implement `From<IpcBytes> for Arc<dyn std::convert::AsRef<[u8]> + Send + Sync>` that casts the inner reference.
    - This helps integration with new `harfrust` crate.

### CLI

This release contains breaking changes to the `cargo zng res` packaging command.

* **Breaking** Changed syntax for `cargo zng res` tools.
    - `.zr-*` becomes ` '*`, e.g. `foo.zr-copy` becomes `foo 'copy`.
    - Tool recursion now resolve from start to end, e.g. `foo.tar.zr-tar.zr-rp` becomes `foo.tar 'rp'tar`.
    - Added `cargo zng res --upgrade-zr <DIR>` command to automatically convert to new syntax.

* **Breaking** Remove "on-final=" support from `cargo zng res`.
    - Added ` 'z` meta tool that skips passes, e.g.: `run 'z'sh` runs the script after all files are staged.
    - Removed `'shf` and `'sfxf`.
    - An error message suggests `cargo zng res --upgrade-zr .` that fixes these issues.
    - Note that the `.zr-apk` tool used to run on final, now it must be defined as `'z'apk`.

* Implement alternate braces (e.g.: "*.{txt,md}") for all `cargo zng res` tools that accept glob patterns.

* Fix `cargo zng res --tool tar` double nesting the root dir when `source = "dir", name = "dir"`.

* `cargo zng --verbose` is now a global argument.

To upgrade run:

```console
cargo install cargo-zng
cargo zng res --upgrade-zr .
```

In a project created with `cargo zng new` you may want to also patch the file associations
for VsCode, you can copy the new definitions from [.vscode/settings.json](https://github.com/zng-ui/zng-template/blob/main/.vscode/settings.json).

*Crates will be available on [crates.io](https://crates.io/crates/zng) once [Publish](https://github.com/zng-ui/zng/actions/runs/bogus) completes.*