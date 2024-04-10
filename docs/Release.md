# Release Process

Release is partially automated, some manual steps must be followed and the automation must be monitored.

To make a release a `zng-ui` project owner needs to follow/monitor these steps:

1. Merge changes into `release` on GitHub.
    * This includes `Cargo.toml` version changes that must be reviewed manually.
    * You can use `cargo semver-checks` to find breaking changes.
    * You can use `git diff v0.0.0 master --name-only -- "**/Cargo.toml"` to find dependency updates.
    * You can use `do publish --bump` to set the versions.
    * Note that if setting manually the `zng-view-prebuilt` needs to have the same version as `zng`.
    * Update changelog header.

2. The release push triggers `.github/workflows/release-1-test-tag.yml`.
    * It will test in all platforms.
    * If all tests pass, it will create a new git tag `v{zng.version}`.

3. The git tag push triggers `.github/workflows/release-2-prebuild-publish.yml`.
    * It will generate new prebuilt binaries.
    * It will make a GitHub release for the new tag with the prebuilt binaries.

4. After you verify the GitHub release worked, manually cargo publish all changed crates.
    * This is fully manual.
    * You can use `do publish --check` to get a list of crates that need to be published.
    * And you can use `do publish --execute` to publish.
        - Note that this command will await the rate limit, 10 minutes per new crate and 1 minute per update.
          For updates there is a burst of 30.
    * Use `do publish --execute --no-burst` after a publish failure to continue.
        - The `--no-burst` flag zeroes the burst rate counter so it will wait the full delay (after the first publish).

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