# Docs TODO

* Normalize docs using guidelines: 
  https://deterministic.space/machine-readable-inline-markdown-code-cocumentation.html
  https://github.com/rust-lang/rfcs/blob/30221dc3e025eb9f8f84ccacbc9622e3a75dff5e/text/1574-more-api-documentation-conventions.md
  https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html
* Review all docs.
    - Mentions of threads in particular.
  
* Review order of properties in docs.
    - Inner module impl are placed first in the docs.
    - Generate some tag in each impl block, use JS to reorder?
    - Still not enough, properties in inner modules have higher priority over those in the same module as the struct.

* Widget image/videos rendering from doc-tests.

* Generate feature docs from `Cargo.toml` comments.