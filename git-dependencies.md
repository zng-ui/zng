# Git Dependencies

Information about dependencies that we currently depend directly from git URL.

When we publish we need to create a fork for these dependencies, for now we use the
git and commit hash.

# Webrender

We use the `webrender`, `webrender_api` and `swgl` crates. They are stable

The current *version* is the latest commit that was included in the Firefox 115 release.

Follow the steps to update:

1 - Use the `[ghsync]` link for the Mozilla central auto-merge commits to find the latest that is in the
    milestone we are interested in.
2 - Change the `rev` in `zero-ui-view-api` and `zero-ui-view` to the new commit.

The current commit is this one: [`https://github.com/servo/webrender/commit/ac434de50b49830032391a042359b7c588b2350b`]