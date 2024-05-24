# Tools

This directory contains crates used to generate content or manage the project.

## `cargo do`

**Do** is a built-in task runner for managing this project, run `cargo do help` or `./do help` for details.

The task runner is implemented as a Rust crate in `cargo-do` and an alias in `/.cargo/config.toml`.
The alias builds the tool silently in the first run, after, it runs without noticeable delay.

Shell script to run `do` are also provided:
 
 * cmd.exe: `do help`.
 * PowerShell: `./do.ps1 help`.
 * Bash: `/.do help`.

### `cargo do install`

The task runner depends on multiple cargo commands, you can run `cargo do install` to see a list of all required 
commands and run `cargo do install --execute` to run the installation commands.

## Adding a Tool

If your tool will probably be used rarely, make a new crate for it, for example the `color-gen` was used to
generate the named colors const Rust code. It is kept just in case another colors module needs to be generated.

### Workspace

Append this code to a tool crate `Cargo.toml` to exclude it from the main workspace.

```toml
[workspace] # Exclude from main workspace
```

### Rust Analyzer

If you want Rust Analyzer (VSCode) to analyze a tool crate you need to manually add it to the 
`"rust-analyzer.linkedProjects"` setting in the  in `.vscode/settings.json`.