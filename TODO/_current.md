# Fix context var map

* Tried:
    - Use `Var::is_contextual`.
    - Have a map for contextual values inside RcMapVar.
    - Too much extra code for an edge case.

* Try:
    - Extend value version to include a context id.
    - Different versions for each context causes re-compute in multi-context edge cases.
    - Pros: Only need to modify ContextVar.
    - Cons: No caching of mapping functions in this case.

# Finish scroll

* Implement ContextVar modify.