* Track version of bounds and render info in the info tree, so we can skip queries that only need to run
   when widgets are moved or hidden.

* A frame is generated for the dummy pipeline just after respawn.
* Integrate frame reuse with frame update, see `Optimizations.md`.