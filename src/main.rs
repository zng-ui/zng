mod app;
mod button;
mod window;

use webrender::api::ColorF;

fn main() {
 let db = ();

    app::App::new()
        .window("window1", ColorF::new(0.1, 0.2, 0.3, 1.0))
        .window("window2", ColorF::new(0.3, 0.2, 0.1, 1.0))
        .run();

Border(Text).click(||{db})
    let w = (Click(Border(Text), fn))
}

trait Ui {
    fn event(){}
    fn render(){}
}

struct Click<T: Ui>(T, fn);

impl Ui for Click {
    fn event(&self) {
        match event { click =>  self.fn()}
    }

    fn render(&self) {
        self.0.render()
    }
}

struct Border<T: Ui>(T);


trait UiBuilder {
    fn click(self) -> Ui;
}

impl<T: Ui> UiBuilder for T {
    fn click(self, fn) -> Ui {
        Click(self, fn)
    }
}

impl Ui for NossaTela {
    fn event() {
        util::is_click(event) {
db
        }
        util::is_click(event) {
db
        }
    }

    fn render() {
        sel
    }
}