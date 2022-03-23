# Docs TODO

* Normalize docs using guidelines: 
  https://deterministic.space/machine-readable-inline-markdown-code-cocumentation.html
  https://github.com/rust-lang/rfcs/blob/30221dc3e025eb9f8f84ccacbc9622e3a75dff5e/text/1574-more-api-documentation-conventions.md
  https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html

* Show widget properties that can be `unset!`.
  - Review how assigned required properties behave.
  - And *required* because capture too.

# Difficult

* Widget image/videos rendering from doc-tests.
* Implements JS rewrites in Rust, targeting the generated doc files.
  - The function must apply the rewrites and remove the custom scripts.
  - Wait until `rustdoc` template is more stable?