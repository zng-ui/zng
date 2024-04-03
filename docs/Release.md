# Release Process

Release is partially automated, some manual steps must be followed and the automation must be monitored.

To make a release a `zng-ui` project owner needs to follow/monitor these steps:

1. Merge changes into `release` on GitHub.
    * This includes `Cargo.toml` version changes that must be done manually.
    * You can use `cargo semver-checks` to find breaking changes.
    * You can use `do publish --bump` to set the versions.
    * Note that if setting manually the `zng-view-prebuilt` needs to have the same version as `zng`.

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
