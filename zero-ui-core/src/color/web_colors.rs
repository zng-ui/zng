//! Named web colors.

use super::Rgba;

macro_rules! rgb {
    ($r:literal, $g:literal, $b:literal) => {
        Rgba {
            red: $r as f32 / 255.,
            green: $g as f32 / 255.,
            blue: $b as f32 / 255.,
            alpha: 1.0,
        }
    };
}

/// <div style="display: inline-block; background-color:#E6E6FA; width:20px; height:20px;"></div> Lavender, <code>#E6E6FA</code>, <code>rgb(230, 230, 250)</code>.
pub const LAVENDER: Rgba = rgb!(230, 230, 250);

/// <div style="display: inline-block; background-color:#D8BFD8; width:20px; height:20px;"></div> Thistle, <code>#D8BFD8</code>, <code>rgb(216, 191, 216)</code>.
pub const THISTLE: Rgba = rgb!(216, 191, 216);

/// <div style="display: inline-block; background-color:#DDA0DD; width:20px; height:20px;"></div> Plum, <code>#DDA0DD</code>, <code>rgb(221, 160, 221)</code>.
pub const PLUM: Rgba = rgb!(221, 160, 221);

/// <div style="display: inline-block; background-color:#EE82EE; width:20px; height:20px;"></div> Violet, <code>#EE82EE</code>, <code>rgb(238, 130, 238)</code>.
pub const VIOLET: Rgba = rgb!(238, 130, 238);

/// <div style="display: inline-block; background-color:#DA70D6; width:20px; height:20px;"></div> Orchid, <code>#DA70D6</code>, <code>rgb(218, 112, 214)</code>.
pub const ORCHID: Rgba = rgb!(218, 112, 214);

/// <div style="display: inline-block; background-color:#FF00FF; width:20px; height:20px;"></div> Fuchsia, <code>#FF00FF</code>, <code>rgb(255, 0, 255)</code>.
pub const FUCHSIA: Rgba = rgb!(255, 0, 255);

/// <div style="display: inline-block; background-color:#FF00FF; width:20px; height:20px;"></div> Magenta, <code>#FF00FF</code>, <code>rgb(255, 0, 255)</code>.
pub const MAGENTA: Rgba = rgb!(255, 0, 255);

/// <div style="display: inline-block; background-color:#BA55D3; width:20px; height:20px;"></div> Medium Orchid, <code>#BA55D3</code>, <code>rgb(186, 85, 211)</code>.
pub const MEDIUM_ORCHID: Rgba = rgb!(186, 85, 211);

/// <div style="display: inline-block; background-color:#9370DB; width:20px; height:20px;"></div> Medium Purple, <code>#9370DB</code>, <code>rgb(147, 112, 219)</code>.
pub const MEDIUM_PURPLE: Rgba = rgb!(147, 112, 219);

/// <div style="display: inline-block; background-color:#8A2BE2; width:20px; height:20px;"></div> Blue Violet, <code>#8A2BE2</code>, <code>rgb(138, 43, 226)</code>.
pub const BLUE_VIOLET: Rgba = rgb!(138, 43, 226);

/// <div style="display: inline-block; background-color:#9400D3; width:20px; height:20px;"></div> Dark Violet, <code>#9400D3</code>, <code>rgb(148, 0, 211)</code>.
pub const DARK_VIOLET: Rgba = rgb!(148, 0, 211);

/// <div style="display: inline-block; background-color:#9932CC; width:20px; height:20px;"></div> Dark Orchid, <code>#9932CC</code>, <code>rgb(153, 50, 204)</code>.
pub const DARK_ORCHID: Rgba = rgb!(153, 50, 204);

/// <div style="display: inline-block; background-color:#8B008B; width:20px; height:20px;"></div> Dark Magenta, <code>#8B008B</code>, <code>rgb(139, 0, 139)</code>.
pub const DARK_MAGENTA: Rgba = rgb!(139, 0, 139);

/// <div style="display: inline-block; background-color:#800080; width:20px; height:20px;"></div> Purple, <code>#800080</code>, <code>rgb(128, 0, 128)</code>.
pub const PURPLE: Rgba = rgb!(128, 0, 128);

/// <div style="display: inline-block; background-color:#4B0082; width:20px; height:20px;"></div> Indigo, <code>#4B0082</code>, <code>rgb(75, 0, 130)</code>.
pub const INDIGO: Rgba = rgb!(75, 0, 130);

/// <div style="display: inline-block; background-color:#483D8B; width:20px; height:20px;"></div> Dark Slate Blue, <code>#483D8B</code>, <code>rgb(72, 61, 139)</code>.
pub const DARK_SLATE_BLUE: Rgba = rgb!(72, 61, 139);

/// <div style="display: inline-block; background-color:#6A5ACD; width:20px; height:20px;"></div> Slate Blue, <code>#6A5ACD</code>, <code>rgb(106, 90, 205)</code>.
pub const SLATE_BLUE: Rgba = rgb!(106, 90, 205);

/// <div style="display: inline-block; background-color:#7B68EE; width:20px; height:20px;"></div> Medium Slate Blue, <code>#7B68EE</code>, <code>rgb(123, 104, 238)</code>.
pub const MEDIUM_SLATE_BLUE: Rgba = rgb!(123, 104, 238);

/// <div style="display: inline-block; background-color:#FFC0CB; width:20px; height:20px;"></div> Pink, <code>#FFC0CB</code>, <code>rgb(255, 192, 203)</code>.
pub const PINK: Rgba = rgb!(255, 192, 203);

/// <div style="display: inline-block; background-color:#FFB6C1; width:20px; height:20px;"></div> Light Pink, <code>#FFB6C1</code>, <code>rgb(255, 182, 193)</code>.
pub const LIGHT_PINK: Rgba = rgb!(255, 182, 193);

/// <div style="display: inline-block; background-color:#FF69B4; width:20px; height:20px;"></div> Hot Pink, <code>#FF69B4</code>, <code>rgb(255, 105, 180)</code>.
pub const HOT_PINK: Rgba = rgb!(255, 105, 180);

/// <div style="display: inline-block; background-color:#FF1493; width:20px; height:20px;"></div> Deep Pink, <code>#FF1493</code>, <code>rgb(255, 20, 147)</code>.
pub const DEEP_PINK: Rgba = rgb!(255, 20, 147);

/// <div style="display: inline-block; background-color:#DB7093; width:20px; height:20px;"></div> Pale Violet Red, <code>#DB7093</code>, <code>rgb(219, 112, 147)</code>.
pub const PALE_VIOLET_RED: Rgba = rgb!(219, 112, 147);

/// <div style="display: inline-block; background-color:#C71585; width:20px; height:20px;"></div> Medium Violet Red, <code>#C71585</code>, <code>rgb(199, 21, 133)</code>.
pub const MEDIUM_VIOLET_RED: Rgba = rgb!(199, 21, 133);

/// <div style="display: inline-block; background-color:#FFA07A; width:20px; height:20px;"></div> Light Salmon, <code>#FFA07A</code>, <code>rgb(255, 160, 122)</code>.
pub const LIGHT_SALMON: Rgba = rgb!(255, 160, 122);

/// <div style="display: inline-block; background-color:#FA8072; width:20px; height:20px;"></div> Salmon, <code>#FA8072</code>, <code>rgb(250, 128, 114)</code>.
pub const SALMON: Rgba = rgb!(250, 128, 114);

/// <div style="display: inline-block; background-color:#E9967A; width:20px; height:20px;"></div> Dark Salmon, <code>#E9967A</code>, <code>rgb(233, 150, 122)</code>.
pub const DARK_SALMON: Rgba = rgb!(233, 150, 122);

/// <div style="display: inline-block; background-color:#F08080; width:20px; height:20px;"></div> Light Coral, <code>#F08080</code>, <code>rgb(240, 128, 128)</code>.
pub const LIGHT_CORAL: Rgba = rgb!(240, 128, 128);

/// <div style="display: inline-block; background-color:#CD5C5C; width:20px; height:20px;"></div> Indian Red, <code>#CD5C5C</code>, <code>rgb(205, 92, 92)</code>.
pub const INDIAN_RED: Rgba = rgb!(205, 92, 92);

/// <div style="display: inline-block; background-color:#DC143C; width:20px; height:20px;"></div> Crimson, <code>#DC143C</code>, <code>rgb(220, 20, 60)</code>.
pub const CRIMSON: Rgba = rgb!(220, 20, 60);

/// <div style="display: inline-block; background-color:#B22222; width:20px; height:20px;"></div> Fire Brick, <code>#B22222</code>, <code>rgb(178, 34, 34)</code>.
pub const FIRE_BRICK: Rgba = rgb!(178, 34, 34);

/// <div style="display: inline-block; background-color:#8B0000; width:20px; height:20px;"></div> Dark Red, <code>#8B0000</code>, <code>rgb(139, 0, 0)</code>.
pub const DARK_RED: Rgba = rgb!(139, 0, 0);

/// <div style="display: inline-block; background-color:#FF0000; width:20px; height:20px;"></div> Red, <code>#FF0000</code>, <code>rgb(255, 0, 0)</code>.
pub const RED: Rgba = rgb!(255, 0, 0);

/// <div style="display: inline-block; background-color:#FF4500; width:20px; height:20px;"></div> Orange Red, <code>#FF4500</code>, <code>rgb(255, 69, 0)</code>.
pub const ORANGE_RED: Rgba = rgb!(255, 69, 0);

/// <div style="display: inline-block; background-color:#FF6347; width:20px; height:20px;"></div> Tomato, <code>#FF6347</code>, <code>rgb(255, 99, 71)</code>.
pub const TOMATO: Rgba = rgb!(255, 99, 71);

/// <div style="display: inline-block; background-color:#FF7F50; width:20px; height:20px;"></div> Coral, <code>#FF7F50</code>, <code>rgb(255, 127, 80)</code>.
pub const CORAL: Rgba = rgb!(255, 127, 80);

/// <div style="display: inline-block; background-color:#FF8C00; width:20px; height:20px;"></div> Dark Orange, <code>#FF8C00</code>, <code>rgb(255, 140, 0)</code>.
pub const DARK_ORANGE: Rgba = rgb!(255, 140, 0);

/// <div style="display: inline-block; background-color:#FFA500; width:20px; height:20px;"></div> Orange, <code>#FFA500</code>, <code>rgb(255, 165, 0)</code>.
pub const ORANGE: Rgba = rgb!(255, 165, 0);

/// <div style="display: inline-block; background-color:#FFFF00; width:20px; height:20px;"></div> Yellow, <code>#FFFF00</code>, <code>rgb(255, 255, 0)</code>.
pub const YELLOW: Rgba = rgb!(255, 255, 0);

/// <div style="display: inline-block; background-color:#FFFFE0; width:20px; height:20px;"></div> Light Yellow, <code>#FFFFE0</code>, <code>rgb(255, 255, 224)</code>.
pub const LIGHT_YELLOW: Rgba = rgb!(255, 255, 224);

/// <div style="display: inline-block; background-color:#FFFACD; width:20px; height:20px;"></div> Lemon Chiffon, <code>#FFFACD</code>, <code>rgb(255, 250, 205)</code>.
pub const LEMON_CHIFFON: Rgba = rgb!(255, 250, 205);

/// <div style="display: inline-block; background-color:#FAFAD2; width:20px; height:20px;"></div> Light Goldenrod Yellow, <code>#FAFAD2</code>, <code>rgb(250, 250, 210)</code>.
pub const LIGHT_GOLDENROD_YELLOW: Rgba = rgb!(250, 250, 210);

/// <div style="display: inline-block; background-color:#FFEFD5; width:20px; height:20px;"></div> Papaya Whip, <code>#FFEFD5</code>, <code>rgb(255, 239, 213)</code>.
pub const PAPAYA_WHIP: Rgba = rgb!(255, 239, 213);

/// <div style="display: inline-block; background-color:#FFE4B5; width:20px; height:20px;"></div> Moccasin, <code>#FFE4B5</code>, <code>rgb(255, 228, 181)</code>.
pub const MOCCASIN: Rgba = rgb!(255, 228, 181);

/// <div style="display: inline-block; background-color:#FFDAB9; width:20px; height:20px;"></div> Peach Puff, <code>#FFDAB9</code>, <code>rgb(255, 218, 185)</code>.
pub const PEACH_PUFF: Rgba = rgb!(255, 218, 185);

/// <div style="display: inline-block; background-color:#EEE8AA; width:20px; height:20px;"></div> Pale Goldenrod, <code>#EEE8AA</code>, <code>rgb(238, 232, 170)</code>.
pub const PALE_GOLDENROD: Rgba = rgb!(238, 232, 170);

/// <div style="display: inline-block; background-color:#F0E68C; width:20px; height:20px;"></div> Khaki, <code>#F0E68C</code>, <code>rgb(240, 230, 140)</code>.
pub const KHAKI: Rgba = rgb!(240, 230, 140);

/// <div style="display: inline-block; background-color:#BDB76B; width:20px; height:20px;"></div> Dark Khaki, <code>#BDB76B</code>, <code>rgb(189, 183, 107)</code>.
pub const DARK_KHAKI: Rgba = rgb!(189, 183, 107);

/// <div style="display: inline-block; background-color:#FFD700; width:20px; height:20px;"></div> Gold, <code>#FFD700</code>, <code>rgb(255, 215, 0)</code>.
pub const GOLD: Rgba = rgb!(255, 215, 0);

/// <div style="display: inline-block; background-color:#FFF8DC; width:20px; height:20px;"></div> Cornsilk, <code>#FFF8DC</code>, <code>rgb(255, 248, 220)</code>.
pub const CORNSILK: Rgba = rgb!(255, 248, 220);

/// <div style="display: inline-block; background-color:#FFEBCD; width:20px; height:20px;"></div> Blanched Almond, <code>#FFEBCD</code>, <code>rgb(255, 235, 205)</code>.
pub const BLANCHED_ALMOND: Rgba = rgb!(255, 235, 205);

/// <div style="display: inline-block; background-color:#FFE4C4; width:20px; height:20px;"></div> Bisque, <code>#FFE4C4</code>, <code>rgb(255, 228, 196)</code>.
pub const BISQUE: Rgba = rgb!(255, 228, 196);

/// <div style="display: inline-block; background-color:#FFDEAD; width:20px; height:20px;"></div> Navajo White, <code>#FFDEAD</code>, <code>rgb(255, 222, 173)</code>.
pub const NAVAJO_WHITE: Rgba = rgb!(255, 222, 173);

/// <div style="display: inline-block; background-color:#F5DEB3; width:20px; height:20px;"></div> Wheat, <code>#F5DEB3</code>, <code>rgb(245, 222, 179)</code>.
pub const WHEAT: Rgba = rgb!(245, 222, 179);

/// <div style="display: inline-block; background-color:#DEB887; width:20px; height:20px;"></div> Burly Wood, <code>#DEB887</code>, <code>rgb(222, 184, 135)</code>.
pub const BURLY_WOOD: Rgba = rgb!(222, 184, 135);

/// <div style="display: inline-block; background-color:#D2B48C; width:20px; height:20px;"></div> Tan, <code>#D2B48C</code>, <code>rgb(210, 180, 140)</code>.
pub const TAN: Rgba = rgb!(210, 180, 140);

/// <div style="display: inline-block; background-color:#BC8F8F; width:20px; height:20px;"></div> Rosy Brown, <code>#BC8F8F</code>, <code>rgb(188, 143, 143)</code>.
pub const ROSY_BROWN: Rgba = rgb!(188, 143, 143);

/// <div style="display: inline-block; background-color:#F4A460; width:20px; height:20px;"></div> Sandy Brown, <code>#F4A460</code>, <code>rgb(244, 164, 96)</code>.
pub const SANDY_BROWN: Rgba = rgb!(244, 164, 96);

/// <div style="display: inline-block; background-color:#DAA520; width:20px; height:20px;"></div> Goldenrod, <code>#DAA520</code>, <code>rgb(218, 165, 32)</code>.
pub const GOLDENROD: Rgba = rgb!(218, 165, 32);

/// <div style="display: inline-block; background-color:#B8860B; width:20px; height:20px;"></div> Dark Goldenrod, <code>#B8860B</code>, <code>rgb(184, 134, 11)</code>.
pub const DARK_GOLDENROD: Rgba = rgb!(184, 134, 11);

/// <div style="display: inline-block; background-color:#CD853F; width:20px; height:20px;"></div> Peru, <code>#CD853F</code>, <code>rgb(205, 133, 63)</code>.
pub const PERU: Rgba = rgb!(205, 133, 63);

/// <div style="display: inline-block; background-color:#D2691E; width:20px; height:20px;"></div> Chocolate, <code>#D2691E</code>, <code>rgb(210, 105, 30)</code>.
pub const CHOCOLATE: Rgba = rgb!(210, 105, 30);

/// <div style="display: inline-block; background-color:#8B4513; width:20px; height:20px;"></div> Saddle Brown, <code>#8B4513</code>, <code>rgb(139, 69, 19)</code>.
pub const SADDLE_BROWN: Rgba = rgb!(139, 69, 19);

/// <div style="display: inline-block; background-color:#A0522D; width:20px; height:20px;"></div> Sienna, <code>#A0522D</code>, <code>rgb(160, 82, 45)</code>.
pub const SIENNA: Rgba = rgb!(160, 82, 45);

/// <div style="display: inline-block; background-color:#A52A2A; width:20px; height:20px;"></div> Brown, <code>#A52A2A</code>, <code>rgb(165, 42, 42)</code>.
pub const BROWN: Rgba = rgb!(165, 42, 42);

/// <div style="display: inline-block; background-color:#800000; width:20px; height:20px;"></div> Maroon, <code>#800000</code>, <code>rgb(128, 0, 0)</code>.
pub const MAROON: Rgba = rgb!(128, 0, 0);

/// <div style="display: inline-block; background-color:#556B2F; width:20px; height:20px;"></div> Dark Olive Green, <code>#556B2F</code>, <code>rgb(85, 107, 47)</code>.
pub const DARK_OLIVE_GREEN: Rgba = rgb!(85, 107, 47);

/// <div style="display: inline-block; background-color:#808000; width:20px; height:20px;"></div> Olive, <code>#808000</code>, <code>rgb(128, 128, 0)</code>.
pub const OLIVE: Rgba = rgb!(128, 128, 0);

/// <div style="display: inline-block; background-color:#6B8E23; width:20px; height:20px;"></div> Olive Drab, <code>#6B8E23</code>, <code>rgb(107, 142, 35)</code>.
pub const OLIVE_DRAB: Rgba = rgb!(107, 142, 35);

/// <div style="display: inline-block; background-color:#9ACD32; width:20px; height:20px;"></div> Yellow Green, <code>#9ACD32</code>, <code>rgb(154, 205, 50)</code>.
pub const YELLOW_GREEN: Rgba = rgb!(154, 205, 50);

/// <div style="display: inline-block; background-color:#32CD32; width:20px; height:20px;"></div> Lime Green, <code>#32CD32</code>, <code>rgb(50, 205, 50)</code>.
pub const LIME_GREEN: Rgba = rgb!(50, 205, 50);

/// <div style="display: inline-block; background-color:#00FF00; width:20px; height:20px;"></div> Lime, <code>#00FF00</code>, <code>rgb(0, 255, 0)</code>.
pub const LIME: Rgba = rgb!(0, 255, 0);

/// <div style="display: inline-block; background-color:#7CFC00; width:20px; height:20px;"></div> Lawn Green, <code>#7CFC00</code>, <code>rgb(124, 252, 0)</code>.
pub const LAWN_GREEN: Rgba = rgb!(124, 252, 0);

/// <div style="display: inline-block; background-color:#7FFF00; width:20px; height:20px;"></div> Chartreuse, <code>#7FFF00</code>, <code>rgb(127, 255, 0)</code>.
pub const CHARTREUSE: Rgba = rgb!(127, 255, 0);

/// <div style="display: inline-block; background-color:#ADFF2F; width:20px; height:20px;"></div> Green Yellow, <code>#ADFF2F</code>, <code>rgb(173, 255, 47)</code>.
pub const GREEN_YELLOW: Rgba = rgb!(173, 255, 47);

/// <div style="display: inline-block; background-color:#00FF7F; width:20px; height:20px;"></div> Spring Green, <code>#00FF7F</code>, <code>rgb(0, 255, 127)</code>.
pub const SPRING_GREEN: Rgba = rgb!(0, 255, 127);

/// <div style="display: inline-block; background-color:#00FA9A; width:20px; height:20px;"></div> Medium Spring Green, <code>#00FA9A</code>, <code>rgb(0, 250, 154)</code>.
pub const MEDIUM_SPRING_GREEN: Rgba = rgb!(0, 250, 154);

/// <div style="display: inline-block; background-color:#90EE90; width:20px; height:20px;"></div> Light Green, <code>#90EE90</code>, <code>rgb(144, 238, 144)</code>.
pub const LIGHT_GREEN: Rgba = rgb!(144, 238, 144);

/// <div style="display: inline-block; background-color:#98FB98; width:20px; height:20px;"></div> Pale Green, <code>#98FB98</code>, <code>rgb(152, 251, 152)</code>.
pub const PALE_GREEN: Rgba = rgb!(152, 251, 152);

/// <div style="display: inline-block; background-color:#8FBC8F; width:20px; height:20px;"></div> Dark Sea Green, <code>#8FBC8F</code>, <code>rgb(143, 188, 143)</code>.
pub const DARK_SEA_GREEN: Rgba = rgb!(143, 188, 143);

/// <div style="display: inline-block; background-color:#3CB371; width:20px; height:20px;"></div> Medium Sea Green, <code>#3CB371</code>, <code>rgb(60, 179, 113)</code>.
pub const MEDIUM_SEA_GREEN: Rgba = rgb!(60, 179, 113);

/// <div style="display: inline-block; background-color:#2E8B57; width:20px; height:20px;"></div> Sea Green, <code>#2E8B57</code>, <code>rgb(46, 139, 87)</code>.
pub const SEA_GREEN: Rgba = rgb!(46, 139, 87);

/// <div style="display: inline-block; background-color:#228B22; width:20px; height:20px;"></div> Forest Green, <code>#228B22</code>, <code>rgb(34, 139, 34)</code>.
pub const FOREST_GREEN: Rgba = rgb!(34, 139, 34);

/// <div style="display: inline-block; background-color:#008000; width:20px; height:20px;"></div> Green, <code>#008000</code>, <code>rgb(0, 128, 0)</code>.
pub const GREEN: Rgba = rgb!(0, 128, 0);

/// <div style="display: inline-block; background-color:#006400; width:20px; height:20px;"></div> Dark Green, <code>#006400</code>, <code>rgb(0, 100, 0)</code>.
pub const DARK_GREEN: Rgba = rgb!(0, 100, 0);

/// <div style="display: inline-block; background-color:#66CDAA; width:20px; height:20px;"></div> Medium Aquamarine, <code>#66CDAA</code>, <code>rgb(102, 205, 170)</code>.
pub const MEDIUM_AQUAMARINE: Rgba = rgb!(102, 205, 170);

/// <div style="display: inline-block; background-color:#00FFFF; width:20px; height:20px;"></div> Aqua, <code>#00FFFF</code>, <code>rgb(0, 255, 255)</code>.
pub const AQUA: Rgba = rgb!(0, 255, 255);

/// <div style="display: inline-block; background-color:#00FFFF; width:20px; height:20px;"></div> Cyan, <code>#00FFFF</code>, <code>rgb(0, 255, 255)</code>.
pub const CYAN: Rgba = rgb!(0, 255, 255);

/// <div style="display: inline-block; background-color:#E0FFFF; width:20px; height:20px;"></div> Light Cyan, <code>#E0FFFF</code>, <code>rgb(224, 255, 255)</code>.
pub const LIGHT_CYAN: Rgba = rgb!(224, 255, 255);

/// <div style="display: inline-block; background-color:#AFEEEE; width:20px; height:20px;"></div> Pale Turquoise, <code>#AFEEEE</code>, <code>rgb(175, 238, 238)</code>.
pub const PALE_TURQUOISE: Rgba = rgb!(175, 238, 238);

/// <div style="display: inline-block; background-color:#7FFFD4; width:20px; height:20px;"></div> Aquamarine, <code>#7FFFD4</code>, <code>rgb(127, 255, 212)</code>.
pub const AQUAMARINE: Rgba = rgb!(127, 255, 212);

/// <div style="display: inline-block; background-color:#40E0D0; width:20px; height:20px;"></div> Turquoise, <code>#40E0D0</code>, <code>rgb(64, 224, 208)</code>.
pub const TURQUOISE: Rgba = rgb!(64, 224, 208);

/// <div style="display: inline-block; background-color:#48D1CC; width:20px; height:20px;"></div> Medium Turquoise, <code>#48D1CC</code>, <code>rgb(72, 209, 204)</code>.
pub const MEDIUM_TURQUOISE: Rgba = rgb!(72, 209, 204);

/// <div style="display: inline-block; background-color:#00CED1; width:20px; height:20px;"></div> Dark Turquoise, <code>#00CED1</code>, <code>rgb(0, 206, 209)</code>.
pub const DARK_TURQUOISE: Rgba = rgb!(0, 206, 209);

/// <div style="display: inline-block; background-color:#20B2AA; width:20px; height:20px;"></div> Light Sea Green, <code>#20B2AA</code>, <code>rgb(32, 178, 170)</code>.
pub const LIGHT_SEA_GREEN: Rgba = rgb!(32, 178, 170);

/// <div style="display: inline-block; background-color:#5F9EA0; width:20px; height:20px;"></div> Cadet Blue, <code>#5F9EA0</code>, <code>rgb(95, 158, 160)</code>.
pub const CADET_BLUE: Rgba = rgb!(95, 158, 160);

/// <div style="display: inline-block; background-color:#008B8B; width:20px; height:20px;"></div> Dark Cyan, <code>#008B8B</code>, <code>rgb(0, 139, 139)</code>.
pub const DARK_CYAN: Rgba = rgb!(0, 139, 139);

/// <div style="display: inline-block; background-color:#008080; width:20px; height:20px;"></div> Teal, <code>#008080</code>, <code>rgb(0, 128, 128)</code>.
pub const TEAL: Rgba = rgb!(0, 128, 128);

/// <div style="display: inline-block; background-color:#B0C4DE; width:20px; height:20px;"></div> Light Steel Blue, <code>#B0C4DE</code>, <code>rgb(176, 196, 222)</code>.
pub const LIGHT_STEEL_BLUE: Rgba = rgb!(176, 196, 222);

/// <div style="display: inline-block; background-color:#B0E0E6; width:20px; height:20px;"></div> Powder Blue, <code>#B0E0E6</code>, <code>rgb(176, 224, 230)</code>.
pub const POWDER_BLUE: Rgba = rgb!(176, 224, 230);

/// <div style="display: inline-block; background-color:#ADD8E6; width:20px; height:20px;"></div> Light Blue, <code>#ADD8E6</code>, <code>rgb(173, 216, 230)</code>.
pub const LIGHT_BLUE: Rgba = rgb!(173, 216, 230);

/// <div style="display: inline-block; background-color:#87CEEB; width:20px; height:20px;"></div> Sky Blue, <code>#87CEEB</code>, <code>rgb(135, 206, 235)</code>.
pub const SKY_BLUE: Rgba = rgb!(135, 206, 235);

/// <div style="display: inline-block; background-color:#87CEFA; width:20px; height:20px;"></div> Light Sky Blue, <code>#87CEFA</code>, <code>rgb(135, 206, 250)</code>.
pub const LIGHT_SKY_BLUE: Rgba = rgb!(135, 206, 250);

/// <div style="display: inline-block; background-color:#00BFFF; width:20px; height:20px;"></div> Deep Sky Blue, <code>#00BFFF</code>, <code>rgb(0, 191, 255)</code>.
pub const DEEP_SKY_BLUE: Rgba = rgb!(0, 191, 255);

/// <div style="display: inline-block; background-color:#1E90FF; width:20px; height:20px;"></div> Dodger Blue, <code>#1E90FF</code>, <code>rgb(30, 144, 255)</code>.
pub const DODGER_BLUE: Rgba = rgb!(30, 144, 255);

/// <div style="display: inline-block; background-color:#6495ED; width:20px; height:20px;"></div> Cornflower Blue, <code>#6495ED</code>, <code>rgb(100, 149, 237)</code>.
pub const CORNFLOWER_BLUE: Rgba = rgb!(100, 149, 237);

/// <div style="display: inline-block; background-color:#4682B4; width:20px; height:20px;"></div> Steel Blue, <code>#4682B4</code>, <code>rgb(70, 130, 180)</code>.
pub const STEEL_BLUE: Rgba = rgb!(70, 130, 180);

/// <div style="display: inline-block; background-color:#4169E1; width:20px; height:20px;"></div> Royal Blue, <code>#4169E1</code>, <code>rgb(65, 105, 225)</code>.
pub const ROYAL_BLUE: Rgba = rgb!(65, 105, 225);

/// <div style="display: inline-block; background-color:#0000FF; width:20px; height:20px;"></div> Blue, <code>#0000FF</code>, <code>rgb(0, 0, 255)</code>.
pub const BLUE: Rgba = rgb!(0, 0, 255);

/// <div style="display: inline-block; background-color:#0000CD; width:20px; height:20px;"></div> Medium Blue, <code>#0000CD</code>, <code>rgb(0, 0, 205)</code>.
pub const MEDIUM_BLUE: Rgba = rgb!(0, 0, 205);

/// <div style="display: inline-block; background-color:#00008B; width:20px; height:20px;"></div> Dark Blue, <code>#00008B</code>, <code>rgb(0, 0, 139)</code>.
pub const DARK_BLUE: Rgba = rgb!(0, 0, 139);

/// <div style="display: inline-block; background-color:#000080; width:20px; height:20px;"></div> Navy, <code>#000080</code>, <code>rgb(0, 0, 128)</code>.
pub const NAVY: Rgba = rgb!(0, 0, 128);

/// <div style="display: inline-block; background-color:#191970; width:20px; height:20px;"></div> Midnight Blue, <code>#191970</code>, <code>rgb(25, 25, 112)</code>.
pub const MIDNIGHT_BLUE: Rgba = rgb!(25, 25, 112);

/// <div style="display: inline-block; background-color:#FFFFFF; width:20px; height:20px;"></div> White, <code>#FFFFFF</code>, <code>rgb(255, 255, 255)</code>.
pub const WHITE: Rgba = rgb!(255, 255, 255);

/// <div style="display: inline-block; background-color:#FFFAFA; width:20px; height:20px;"></div> Snow, <code>#FFFAFA</code>, <code>rgb(255, 250, 250)</code>.
pub const SNOW: Rgba = rgb!(255, 250, 250);

/// <div style="display: inline-block; background-color:#F0FFF0; width:20px; height:20px;"></div> Honeydew, <code>#F0FFF0</code>, <code>rgb(240, 255, 240)</code>.
pub const HONEYDEW: Rgba = rgb!(240, 255, 240);

/// <div style="display: inline-block; background-color:#F5FFFA; width:20px; height:20px;"></div> Mint Cream, <code>#F5FFFA</code>, <code>rgb(245, 255, 250)</code>.
pub const MINT_CREAM: Rgba = rgb!(245, 255, 250);

/// <div style="display: inline-block; background-color:#F0FFFF; width:20px; height:20px;"></div> Azure, <code>#F0FFFF</code>, <code>rgb(240, 255, 255)</code>.
pub const AZURE: Rgba = rgb!(240, 255, 255);

/// <div style="display: inline-block; background-color:#F0F8FF; width:20px; height:20px;"></div> Alice Blue, <code>#F0F8FF</code>, <code>rgb(240, 248, 255)</code>.
pub const ALICE_BLUE: Rgba = rgb!(240, 248, 255);

/// <div style="display: inline-block; background-color:#F8F8FF; width:20px; height:20px;"></div> Ghost White, <code>#F8F8FF</code>, <code>rgb(248, 248, 255)</code>.
pub const GHOST_WHITE: Rgba = rgb!(248, 248, 255);

/// <div style="display: inline-block; background-color:#F5F5F5; width:20px; height:20px;"></div> White Smoke, <code>#F5F5F5</code>, <code>rgb(245, 245, 245)</code>.
pub const WHITE_SMOKE: Rgba = rgb!(245, 245, 245);

/// <div style="display: inline-block; background-color:#FFF5EE; width:20px; height:20px;"></div> Seashell, <code>#FFF5EE</code>, <code>rgb(255, 245, 238)</code>.
pub const SEASHELL: Rgba = rgb!(255, 245, 238);

/// <div style="display: inline-block; background-color:#F5F5DC; width:20px; height:20px;"></div> Beige, <code>#F5F5DC</code>, <code>rgb(245, 245, 220)</code>.
pub const BEIGE: Rgba = rgb!(245, 245, 220);

/// <div style="display: inline-block; background-color:#FDF5E6; width:20px; height:20px;"></div> Old Lace, <code>#FDF5E6</code>, <code>rgb(253, 245, 230)</code>.
pub const OLD_LACE: Rgba = rgb!(253, 245, 230);

/// <div style="display: inline-block; background-color:#FFFAF0; width:20px; height:20px;"></div> Floral White, <code>#FFFAF0</code>, <code>rgb(255, 250, 240)</code>.
pub const FLORAL_WHITE: Rgba = rgb!(255, 250, 240);

/// <div style="display: inline-block; background-color:#FFFFF0; width:20px; height:20px;"></div> Ivory, <code>#FFFFF0</code>, <code>rgb(255, 255, 240)</code>.
pub const IVORY: Rgba = rgb!(255, 255, 240);

/// <div style="display: inline-block; background-color:#FAEBD7; width:20px; height:20px;"></div> Antique White, <code>#FAEBD7</code>, <code>rgb(250, 235, 215)</code>.
pub const ANTIQUE_WHITE: Rgba = rgb!(250, 235, 215);

/// <div style="display: inline-block; background-color:#FAF0E6; width:20px; height:20px;"></div> Linen, <code>#FAF0E6</code>, <code>rgb(250, 240, 230)</code>.
pub const LINEN: Rgba = rgb!(250, 240, 230);

/// <div style="display: inline-block; background-color:#FFF0F5; width:20px; height:20px;"></div> Lavender Blush, <code>#FFF0F5</code>, <code>rgb(255, 240, 245)</code>.
pub const LAVENDER_BLUSH: Rgba = rgb!(255, 240, 245);

/// <div style="display: inline-block; background-color:#FFE4E1; width:20px; height:20px;"></div> Misty Rose, <code>#FFE4E1</code>, <code>rgb(255, 228, 225)</code>.
pub const MISTY_ROSE: Rgba = rgb!(255, 228, 225);

/// <div style="display: inline-block; background-color:#DCDCDC; width:20px; height:20px;"></div> Gainsboro, <code>#DCDCDC</code>, <code>rgb(220, 220, 220)</code>.
pub const GAINSBORO: Rgba = rgb!(220, 220, 220);

/// <div style="display: inline-block; background-color:#D3D3D3; width:20px; height:20px;"></div> Light Gray, <code>#D3D3D3</code>, <code>rgb(211, 211, 211)</code>.
pub const LIGHT_GRAY: Rgba = rgb!(211, 211, 211);

/// <div style="display: inline-block; background-color:#C0C0C0; width:20px; height:20px;"></div> Silver, <code>#C0C0C0</code>, <code>rgb(192, 192, 192)</code>.
pub const SILVER: Rgba = rgb!(192, 192, 192);

/// <div style="display: inline-block; background-color:#A9A9A9; width:20px; height:20px;"></div> Dark Gray, <code>#A9A9A9</code>, <code>rgb(169, 169, 169)</code>.
pub const DARK_GRAY: Rgba = rgb!(169, 169, 169);

/// <div style="display: inline-block; background-color:#808080; width:20px; height:20px;"></div> Gray, <code>#808080</code>, <code>rgb(128, 128, 128)</code>.
pub const GRAY: Rgba = rgb!(128, 128, 128);

/// <div style="display: inline-block; background-color:#696969; width:20px; height:20px;"></div> Dim Gray, <code>#696969</code>, <code>rgb(105, 105, 105)</code>.
pub const DIM_GRAY: Rgba = rgb!(105, 105, 105);

/// <div style="display: inline-block; background-color:#778899; width:20px; height:20px;"></div> Light Slate Gray, <code>#778899</code>, <code>rgb(119, 136, 153)</code>.
pub const LIGHT_SLATE_GRAY: Rgba = rgb!(119, 136, 153);

/// <div style="display: inline-block; background-color:#708090; width:20px; height:20px;"></div> Slate Gray, <code>#708090</code>, <code>rgb(112, 128, 144)</code>.
pub const SLATE_GRAY: Rgba = rgb!(112, 128, 144);

/// <div style="display: inline-block; background-color:#2F4F4F; width:20px; height:20px;"></div> Dark Slate Gray, <code>#2F4F4F</code>, <code>rgb(47, 79, 79)</code>.
pub const DARK_SLATE_GRAY: Rgba = rgb!(47, 79, 79);

/// <div style="display: inline-block; background-color:#000000; width:20px; height:20px;"></div> Black, <code>#000000</code>, <code>rgb(0, 0, 0)</code>.
pub const BLACK: Rgba = rgb!(0, 0, 0);
