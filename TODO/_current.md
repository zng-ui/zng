* Better render reuse, see `Optimizations.md`.

* A frame is generated for the dummy pipeline just after respawn.
* Update widget transforms on reuse.
 * Currently the icon example's scrollbar gets drawn in different positions if you alternate between mousing over the scrollbar and mousing over the icons, after having scrolled any amount.