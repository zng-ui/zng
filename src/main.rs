mod app;
mod button;
mod ui;
mod window;

use ui::*;
use webrender::api::*;

fn main() {
    let r_color = ColorF::new(0.2, 0.4, 0.1, 1.);
    let r_size = LayoutSize::new(554., 50.);
    app::App::new()
        .window(
            "window1",
            ColorF::new(0.1, 0.2, 0.3, 1.0),
            center(size(Rect::new(r_color), r_size)),
        )
        .window(
            "window2",
            ColorF::new(0.3, 0.2, 0.1, 1.0),
            Rect::new(r_color).size(r_size).center(),
        )
        .run();
}

pub fn center<T: Ui>(child: T) -> Centered<T> {
    Centered::new(child)
}
pub fn size<T: Ui>(child: T, size: LayoutSize) -> Sized<T> {
    Sized::new(child, size)
}

trait SizedExt: Ui + std::marker::Sized {
    fn size(self, size: LayoutSize) -> Sized<Self>;
}
impl<T: Ui> SizedExt for T {
    fn size(self, size: LayoutSize) -> Sized<Self> {
        Sized::new(self, size)
    }
}

trait CenteredExt: Ui + std::marker::Sized {
    fn center(self) -> Centered<Self>;
}
impl<T: Ui> CenteredExt for T {
    fn center(self) -> Centered<Self> {
        Centered::new(self)
    }
}

struct Rect {
    color: ColorF,
}

impl Rect {
    pub fn new(color: ColorF) -> Self {
        Rect { color }
    }
}

impl Ui for Rect {
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        if available_size.width.is_infinite() {
            available_size.width = 0.;
        }

        if available_size.height.is_infinite() {
            available_size.height = 0.;
        }

        available_size
    }

    fn render(&self, c: RenderContext) {
        let lpi = LayoutPrimitiveInfo::new(LayoutRect::from_size(c.final_size()));
        let sci = SpaceAndClipInfo {
            spatial_id: c.spatial_id(),
            clip_id: ClipId::root(c.spatial_id().pipeline_id()),
        };
        c.builder.push_rect(&lpi, &sci, self.color);
    }
}

pub struct Sized<T: Ui> {
    child: T,
    size: LayoutSize,
}

impl<T: Ui> Sized<T> {
    pub fn new(child: T, size: LayoutSize) -> Self {
        Sized { child, size }
    }
}

impl<T: Ui> Ui for Sized<T> {
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.child.measure(self.size);
        self.size
    }

    fn render(&self, c: RenderContext) {
        self.child.render(c)
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.child.arrange(final_size)
    }
}

pub struct Centered<T: Ui> {
    child: T,
    child_size: LayoutSize,
}

impl<T: Ui> Centered<T> {
    pub fn new(child: T) -> Self {
        Centered {
            child,
            child_size: LayoutSize::default(),
        }
    }
}

impl<T: Ui> Ui for Centered<T> {
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        self.child_size = self.child.measure(available_size);

        if available_size.width.is_infinite() {
            available_size.width = self.child_size.width;
        }

        if available_size.height.is_infinite() {
            available_size.height = self.child_size.height;
        }

        available_size
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.child.arrange(final_size)
    }

    fn render(&self, mut c: RenderContext) {
        //centered = final_rect
        let centered = LayoutRect::new(
            LayoutPoint::new(
                (c.final_size().width - self.child_size.width) / 2.,
                (c.final_size().height - self.child_size.height) / 2.,
            ),
            self.child_size,
        );
        c.push_child(&self.child, &centered);
    }
}
