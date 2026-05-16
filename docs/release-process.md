# Release Process

This document covers how to bump versions and release the prebuilt binaries and crates. Release is partially automated, some manual steps must be followed and the automation must be monitored.

To make a release a `zng-ui` project owner needs to follow/monitor these steps:

1. Select a commit from main branch to be the next release head.
   * All significant changes are documented in the CHANGELOG.
   * Auto README sections are updated, call `do doc --readme` and `do doc --readme-examples` to be sure.
   * All CI tests pass.
   * All examples look ok, call `do run --all` and manually test each.

2. Update versions.
    * This includes `Cargo.toml` version changes that must be reviewed manually.
    * You can use `do publish --diff` to get a list of crates and files that changed.
    * You can use `cargo semver-checks` to find breaking changes.
        - **Warning** This tool does not find all breaking changes, specially it does not detect usage
          of dependency types in the public API when that dependency was upgraded.
    * You can use `do publish --bump` to set the versions, update Cargo.toml doc examples and close the changelog.
    * Note that if setting manually the `zng-view-prebuilt` needs to have the same version as `zng`.

3. Rebase or merge changes into the "release" branch.
    * The release push triggers `.github/workflows/release.yml`.
    * It will test in all platforms.
    * It will build doc for Ubuntu and prebuild for all platforms.
    * If all tests pass it will: 
        - Create a new git tag `v{zng.version}`.
        - Publish doc to `zng-ui/zng-ui.github.io`.
        - And make a GitHub release with the prebuilt libraries.
    * If GitHub release and docs update:
        - It will publish to crates.io using `do publish --execute`.

4. Tests after publish
   * Make a test crate that depends on the previous minor version of `zng`, it must still build.
      You can use `cargo do test --published` to automatically do this.
   * Update and test the template project (`zng-template`) repository.
   * Update the <https://zng-ui.github.io/> header if the reused Rust docs CSS files have changed.
   * Check if the custom docs in a [widget page](https://zng-ui.github.io/doc/zng/text/struct.Text.html) still load properly.

## Webrender

Webrender updates where not published for a long time so we published our own fork in <https://github.com/zng-ui/zng-webrender>.

In zng-0.22.0 we went back to depending on the original Webrender as it has updated.

In zng-0.022.5 we depend on zng-webrender again as it fixes a critical ANGLE bug.

To update the fork crates:

* Checkout the latest published branch from upstream <https://github.com/servo/webrender>.
* Checkout a new branch named `release-0.v.v`.
* Add a `FORK.md` file and set it as the readme for all crates.
* Apply custom patch.
* Rename all patched crates on the list:
```
zng-webrender-build
zng-peek-poke-derive
zng-peek-poke
zng-glsl-to-cxx
zng-wr-malloc-size-of
zng-swgl
zng-webrender-api
zng-wr-glyph-rasterizer
zng-webrender
```
* Bump patched crates version.
* Manually publish each crate.
* Publish new branch to Github.
* If publishing new crates also set the crate owner `cargo owner --add github:zng-ui:owners [CRATE]`.
