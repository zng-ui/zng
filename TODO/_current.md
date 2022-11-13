* Restore parent window when child is restored. 
    - Need to bring_to_front, parent, siblings, self on focus of a child.
* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?