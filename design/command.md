# Command

A command is an app function that can be triggered by shortcut or a button or a menu, it can have an associated 
label, icon and description. Commands can be disabled and/or hidden depending of the app state.

## States

A command can be in 3 states:

1. Enabled - The command executes when the associated shortcut is pressed, widgets that run the command
               are visible and enabled.
2. Disabled - The command does not execute and does not stop the propagation of the shortcut press, widgets
               that run the command are visible but disabled.
3. Collapsed - The command is disabled and widgets that run the command are not visible (collapsed).

The difference between Disabled and Collapsed is that a disabled command is relevant in the overall app state even
if it can be run in the exact current state, a collapsed command is not relevant in the overall app state.

## Observations

* The app state that enables commands includes the current focused widget state, at least in some cases, like a command
  "Paste (Ctrl+V)" needs a text box focused, but we want a menu or toolbar button to paste on the focused text box on click,
  so we have to consider if the returns focus target when the focus is in an ALT scope.
* The separator line in menu layouts is dynamic linked with what command items is not collapsed, if all items in between
  separators is collapsed only as single separator should show.
  - The same goes for menu groups, that is those that just open another menu list, they should not be visible is no child is visible.
  - Does that mean the the two separator lines can be thought as an inlined group?
  - This is more relevant for the menu design, but we need to think about how `Visibility::Collapsed` is tracked, it would be more easy
    if all zero sized widgets are `Collapsed`, today `WidgetVisibilityExt` only the property value.

## Other Implementations

  In VsCode a command is a 'string' named function, it has a custom input and output type and a single handler that is part
of the command declaration. Only some of the "commands" are user facing, and those gain some extra metadata like a label
and can be bound to shortcuts. That is a data entry in a different place, only the 'string' name connects it to the command
declaration. See https://code.visualstudio.com/api/extension-guides/command.

  In Visual Studio a command is a class `MenuCommand`, it contains an ID and the command state an a single handler, you can inherit
from this class to expose the handler for anyone. Also of interest the state includes an extra state `Checked`. There is one built-in
derived class `DesignerVerb` witch adds a `Text` and `Description`. These commands can also receive an optional input of any type, but
the command state does not consider the input, its always enabled or not.

   In WPF there is an `ICommand` interface that only provides two methods `void Execute(object)` and `bool CanExecute(object)`, this
 interface can be plugged in various widgets in a single property `Command` that will then control if the widget is enabled. There is
 `RoutedCommand` class that implements the interface but does not execute the command it only raises events. And finally a class
 `RoutedUICommand` inherits from `RoutedCommand` and adds a `Text` property. Handlers for the `RoutedCommand` can be added in any place of
 the UI tree, the same for shortcut objects, See: https://docs.microsoft.com/en-us/dotnet/desktop/wpf/advanced/commanding-overview?view=netframeworkdesktop-4.8
   There are so many problems caused by this design that Microsoft added a document explaining how to work around then, https://docs.microsoft.com/en-us/previous-versions/msp-n-p/ff921126(v=pandp.20).

## Requirements

There are multiple requirements all grouped under the same label "Command", we expect:

* A *named* handler that can say if it can execute or if the command is even relevant.
* An event that can say if any handler can execute it or if any handler thinks its relevant.
* Associated metadata that can quickly make a button or menu item get the command look.
* Associated shortcut that fires the command.
* Command handlers can receive arguments.
* The command state can change depending of the arguments.

And when it comes to the command state:

* If will depend on the focused widget only in some cases.
* It will also depend in what part of the screen is visible or *selected*.
* It will also depend only on the data layer, the only change in the screen being the command associated widgets getting enabled.

And the shortcuts:

* A shortcut may be set only in a context.
* Multiple commands can end-up with the same shortcut.

And the metadata:

* Usually a label and a longer description is provided, but we may want to allow other stuff, like an icon.
* Maybe add an StateMap to the type to allow any number of arbitrary metadata.
* Some commands can be read-only, maybe the same ones that only have a single handler.
* Some widgets associated with a command can still prefer to set their own label or icon.