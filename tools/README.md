# Tools

This directory contains crates used to generate content or manage the project.

## Workspace

Append this code to a tool crate `Cargo.toml` to exclude it from the main workspace.

```toml
[workspace] # Exclude from main workspace
```

## Rust Analyzer

If you want Rust Analyzer (VSCode) to analyze a tool crate you need to manually add it to the 
`"rust-analyzer.linkedProjects"` setting in the  in `.vscode/settings.json`.