# Contributing to Zng

Thank you for your interest in contributing to Zng! There are many ways to contribute
and we appreciate all of them.

### Review Our English

None of the core developers are native speakers, if you see any grammar mistake, typo 
or sentence that doesn't read right don't hesitate to create a pull request.

### Report a Bug

Create an issue, provide a minimal reproducible example (MRE) that triggers the issue, 
if manual interaction needs to happen, provide a list of steps to follow to cause the issue. 

The issue must happen in the latest release or newer (master branch) and after running `cargo update`.

### Close a Todo

The project is under active development, the [todo] issue label tracks things that need to 
be implemented. To claim a todo open a draft pull request that references the issue or leave a comment.

If you need help getting started leave a comment under the todo issue or start a [new discussion].

## Project Overview

The [`examples/README.md`] doc provides an example list with descriptions.

The [`crates/README.md`] doc provides an overview of the public crates.

The [`tools/README.md`] doc provides an overview of the tools used to manage the project, in 
particular the `cargo do` tool that you will need to use for testing.

The [`tests/README.md`] doc provides an overview of integration and macro tests.

The [`docs`] directory contains more dev docs.

### VSCode & Rust Analyzer

Some workspace settings are included in the repository, in particular, `rust-analyzer` "checkOnSave" 
and runnables are redirected to the `do` tool.

[`API docs`]: https://docs.rs/zng
[`cargo-expand`]: https://github.com/dtolnay/cargo-expand
[`cargo-asm`]: https://github.com/gnzlbg/cargo-asm

[todo]: https://github.com/zng-ui/zng/issues?q=is%3Aissue+is%3Aopen+label%3Atodo
[new discussion]: https://github.com/zng-ui/zng/discussions/new?category=general
[`examples/README.md`]: examples/README.md
[`crates/README.md`]: crates/README.md
[`tools/README.md`]: tools/README.md
[`tests/README.md`]: tests/README.md
[`docs`]: docs