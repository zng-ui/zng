# Requirements:
- Wait for framerender to return from resize event. (resized frame ready)
- 

# Variables:
- monitor 
- size, auto_size, min_size, max_size
- position
- actual_position, actual_monitor, actual_size
- start_position

## On Layout Init:
- monitor: Is used because we need to know the monitor's size and the monitor's scale factor (dpi) before we open the window.
- - monitor size is used for layout context (i.e. a window with a width of 99% means 99% of the monitor size)

- size: Is used but overwritten by auto_size
- auto_size: The window is sized to fit the window's content (can be just width or height)

- min_size & max_size: Constrains size and auto_size to these limits.
- - max_size is the available_size for auto_size

- actual_size: Is the computed size (if size is 90% of the screen, actual_size is that value in pixels before DPI multiplication)
- actual_monitor: Is set by the monitor selected by the monitor variable.

- actual_position: Is computed from actual_size and monitor size (i.e. if the start_position is CenterMonitor)
- position: Is used but overwritten by start_position

## On Subsequent Layouts:
- actual_size: Is used unless auto_size is set.

## On Variable Update:
- when monitor changes: See WindowVars::monitor()
- when size changes: If not being overwritten by auto_size then actual_size is set and a layout is requested.

- when min_size or max_size changes: 
- - Constrains actual_size to these limits and requests layout if changed. 
- - Or if auto_size then a layout is requested.
- - Updates winit constraints.

- when position changes: actual_position is computed and the window moved.

## On End-User Resized or Moved the window:
- actual_position: Assigned from system.
- actual_size: Assigned from system already constrained by min_size and max_size.
- actual_monitor: Computed by intersection between window and monitors? (the monitor area that contains more than half of the window?)


# What information the layout function needs:
- What values need to be updated
 * 
- What values need to be ignored
 * size? (always ignored unless the variable is updated?)


 