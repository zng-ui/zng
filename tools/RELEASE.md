# Release Process

This document covers how to bump versions and release the prebuilt binaries and crates. Release is partially automated, some manual steps must be followed and the automation must be monitored.

To make a release a `zng-ui` project owner needs to follow/monitor these steps:

1. Select a master commit to be the next release head.
   * All significant changes are documented in the CHANGELOG.
   * Auto README sections are updated, call `do doc --readme` and `do doc --readme-examples` to be sure.
   * All CI tests pass.
   * All examples look ok, call `do run --all` and manually test each.

2. Update versions.
    * This includes `Cargo.toml` version changes that must be reviewed manually.
    * You can use `do publish --diff` to get a list of crates and files that changed.
    * You can use `cargo semver-checks` to find breaking changes.
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

## Webrender

Webrender is not published so we maintain our own fork in <https://github.com/zng-ui/zng-webrender>. These crates mostly untouched,
for now we just rename and remove a dependency that has a security advisory (`time`, does not impact Webrender, but shows in `cargo audit`).

To update these crates:

* Merge from upstream <https://github.com/servo/webrender>.
* Manually increment the minor version of each crate that changed.
* We depend on `zng-webrender`, `zng-swgl` and all local dependencies of these crates. As of last publish these are:

```
zng-peek-poke-derive
zng-peek-poke
zng-glsl-to-cxx
zng-wr-malloc-size-of
zng-webrender-build
zng-swgl
zng-webrender-api
zng-wr-glyph-rasterizer
zng-webrender
```

* Push changes to GitHub.
* Test the `zng` project, both `do test` and a manual review using `do prebuild` and `do run -all`.
* Manually publish each crate.

* If publishing new crates also set the crate owner `cargo owner --add github:zng-ui:owners [CRATE]`.