use zero_ui::core::app::App;
use zero_ui::core::text::font_loading2::{FontManager, Fonts};
use zero_ui::core::text::FontName;

fn main() {
    // only load font services.
    App::empty().extend(FontManager::default()).run(|ctx| {
        let font = ctx.services.req::<Fonts>().get_normal(&FontName::monospace()).unwrap();
        println!("{:#?}", font.metrics());
    });
}
