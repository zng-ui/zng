# Submodule Dependencies

The `Cargo.toml` git path dependencies are not supported by [`crates.io`], so if we need to depend
on a git repository we load then in this directory as a git submodule and reference it using a relative path.

Usually if a crate is not published it is not ready for use, an exception to this is Mozilla crates, they are
confident enough to use the code in the stable Firefox release but the crate version is not changed and some
crates not even published.

If the submodule is a Cargo workspace remember to exclude it in the root `Cargo.toml` workspace.

# Webrender

From the `./webrender` submodule we use the `webrender`, `webrender_api` and `swgl` crates.

The current *version* is the latest commit that was included in the Firefox 96.0 release, currently we are manually
searching this commit, it would be nice to have `do` find the latest commit for the latest Firefox stable TODO.

The steps to update manually:

1 - Follow the `[ghsync]` link for the Mozilla central auto-merge commits to find the latest that is in the
    milestone we are interested in.
2 - Checkout this commit in the `./webrender` sub-module.

The current commit is this one: [`https://github.com/servo/webrender/commit/95ca1671b55a4e8bea697a339652103705aeb328`]