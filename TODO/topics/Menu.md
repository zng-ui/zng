# Menu TODO

* Radio button style.
    - Some old apps have this (View->Language in IDM for example).
    - Inherit toggle::selector across sub-menus.
* Mnemonics.
    - Also called access key, accelerator key.
    - Only manually set in all frameworks reviewed.
    - No reason we can't auto-generate this.
    - Key can be reused in different contexts.
        - Visual Studio has a different context for each popup.
        - WPF lets us define AccessKey scope.
            - Different than menu stuff?