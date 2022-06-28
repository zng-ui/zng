* Focus manager is doing a lot of processing after each render in the icon example with full icons.
    - Its the `enabled_nav` query.
    - All nav queries are linear searches, but usually we only do one after the user requests it.
    - We don't need to know the exact "next_tab" to now that we can tab, do faster queries?
    - Can we speedup the full query too? Some sort of map or quad-tree?

* A frame is generated for the dummy pipeline just after respawn.
* Integrate frame reuse with frame update, see `Optimizations.md`.