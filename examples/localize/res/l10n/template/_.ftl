### Localize Example
### This standalone comment is added to all scraped template files.
### This standalone comment is only added to the default file.
### Another standalone comment, also added to the top of the template file.

# icon:
#     first syllable of "Localize"
# title:
#     Main window title
window =
    .icon = Lo
    .title = Localize Example (template)

## Commands

LOCALIZED_CMD =
    .info = Localized in the default file
    .name = Localized

PRIVATE_LOCALIZED_CMD =
    .info = Private command, public localization text
    .name = Private

# the [<ENTER>] text must not be translated, it is replaced by a localized shortcut text widget
press-shortcut-msg = Press the new shortcut and then press [<ENTER>]

## Example Section

# button sets "click-count"
button = Button

example-cmds = Example Commands:

example-shortcuts = Example Shortcuts:

no-shortcut = no shortcut
