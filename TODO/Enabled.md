# IsEnabled TODO

The `IsEnabled` tracks the enabled status of widgets, it is also a test-case of the utility of WidgetInfo metadata and
*read-only* context vars that are wrapped in a struct.

# Problems

* Subscribing to `IsEnabled` is not very ergonomic, and it is used in a lot of state probe properties.