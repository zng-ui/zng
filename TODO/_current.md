# Final Changes


* Review relative values in corner radius.
* Reimplement background/foreground to have their own corners clip and *border-align* that defines what amount of the 
  `border_offsets` they are affected by.
* Review all `fill` properties, they must not affect the positioning of the content.
* Review child priority integration with `WidgetLayout`, what happens when we add a border in child { }?
* Review `side_offsets`, needs to work like an invisible border? 
* Review `clip_to_bounds`.
* Review docs of property and functions that use the term "inner".
* Review container, padding and align only works with widget child but it accepts UiNode child.