mod button;
mod window;

use webrender::api::*;
use window::Window;

fn main() {
    let mut wins = vec![
        Window::new("window1", ColorF::new(0.1, 0.2, 0.3, 1.0)),
        Window::new("window2", ColorF::new(0.3, 0.2, 0.1, 1.0)),
    ];

    while !wins.is_empty() {
        let mut i = 0;
        while i != wins.len() {
            if wins[i].tick() {
                let win = wins.remove(i);
                win.deinit()
            } else {
                i += 1;
            }
        }
    }
}
