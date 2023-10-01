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

/// <span style="display: inline-block; background-color:#E6E6FA; width:20px; height:20px;"></span> Lavender, `#E6E6FA`, `rgb(230, 230, 250)`.
pub const LAVENDER: Rgba = rgb!(230, 230, 250);

/// <span style="display: inline-block; background-color:#D8BFD8; width:20px; height:20px;"></span> Thistle, `#D8BFD8`, `rgb(216, 191, 216)`.
pub const THISTLE: Rgba = rgb!(216, 191, 216);

/// <span style="display: inline-block; background-color:#DDA0DD; width:20px; height:20px;"></span> Plum, `#DDA0DD`, `rgb(221, 160, 221)`.
pub const PLUM: Rgba = rgb!(221, 160, 221);

/// <span style="display: inline-block; background-color:#EE82EE; width:20px; height:20px;"></span> Violet, `#EE82EE`, `rgb(238, 130, 238)`.
pub const VIOLET: Rgba = rgb!(238, 130, 238);

/// <span style="display: inline-block; background-color:#DA70D6; width:20px; height:20px;"></span> Orchid, `#DA70D6`, `rgb(218, 112, 214)`.
pub const ORCHID: Rgba = rgb!(218, 112, 214);

/// <span style="display: inline-block; background-color:#FF00FF; width:20px; height:20px;"></span> Fuchsia, `#FF00FF`, `rgb(255, 0, 255)`.
pub const FUCHSIA: Rgba = rgb!(255, 0, 255);

/// <span style="display: inline-block; background-color:#FF00FF; width:20px; height:20px;"></span> Magenta, `#FF00FF`, `rgb(255, 0, 255)`.
pub const MAGENTA: Rgba = rgb!(255, 0, 255);

/// <span style="display: inline-block; background-color:#BA55D3; width:20px; height:20px;"></span> Medium Orchid, `#BA55D3`, `rgb(186, 85, 211)`.
pub const MEDIUM_ORCHID: Rgba = rgb!(186, 85, 211);

/// <span style="display: inline-block; background-color:#9370DB; width:20px; height:20px;"></span> Medium Purple, `#9370DB`, `rgb(147, 112, 219)`.
pub const MEDIUM_PURPLE: Rgba = rgb!(147, 112, 219);

/// <span style="display: inline-block; background-color:#8A2BE2; width:20px; height:20px;"></span> Blue Violet, `#8A2BE2`, `rgb(138, 43, 226)`.
pub const BLUE_VIOLET: Rgba = rgb!(138, 43, 226);

/// <span style="display: inline-block; background-color:#9400D3; width:20px; height:20px;"></span> Dark Violet, `#9400D3`, `rgb(148, 0, 211)`.
pub const DARK_VIOLET: Rgba = rgb!(148, 0, 211);

/// <span style="display: inline-block; background-color:#9932CC; width:20px; height:20px;"></span> Dark Orchid, `#9932CC`, `rgb(153, 50, 204)`.
pub const DARK_ORCHID: Rgba = rgb!(153, 50, 204);

/// <span style="display: inline-block; background-color:#8B008B; width:20px; height:20px;"></span> Dark Magenta, `#8B008B`, `rgb(139, 0, 139)`.
pub const DARK_MAGENTA: Rgba = rgb!(139, 0, 139);

/// <span style="display: inline-block; background-color:#800080; width:20px; height:20px;"></span> Purple, `#800080`, `rgb(128, 0, 128)`.
pub const PURPLE: Rgba = rgb!(128, 0, 128);

/// <span style="display: inline-block; background-color:#4B0082; width:20px; height:20px;"></span> Indigo, `#4B0082`, `rgb(75, 0, 130)`.
pub const INDIGO: Rgba = rgb!(75, 0, 130);

/// <span style="display: inline-block; background-color:#483D8B; width:20px; height:20px;"></span> Dark Slate Blue, `#483D8B`, `rgb(72, 61, 139)`.
pub const DARK_SLATE_BLUE: Rgba = rgb!(72, 61, 139);

/// <span style="display: inline-block; background-color:#6A5ACD; width:20px; height:20px;"></span> Slate Blue, `#6A5ACD`, `rgb(106, 90, 205)`.
pub const SLATE_BLUE: Rgba = rgb!(106, 90, 205);

/// <span style="display: inline-block; background-color:#7B68EE; width:20px; height:20px;"></span> Medium Slate Blue, `#7B68EE`, `rgb(123, 104, 238)`.
pub const MEDIUM_SLATE_BLUE: Rgba = rgb!(123, 104, 238);

/// <span style="display: inline-block; background-color:#FFC0CB; width:20px; height:20px;"></span> Pink, `#FFC0CB`, `rgb(255, 192, 203)`.
pub const PINK: Rgba = rgb!(255, 192, 203);

/// <span style="display: inline-block; background-color:#FFB6C1; width:20px; height:20px;"></span> Light Pink, `#FFB6C1`, `rgb(255, 182, 193)`.
pub const LIGHT_PINK: Rgba = rgb!(255, 182, 193);

/// <span style="display: inline-block; background-color:#FF69B4; width:20px; height:20px;"></span> Hot Pink, `#FF69B4`, `rgb(255, 105, 180)`.
pub const HOT_PINK: Rgba = rgb!(255, 105, 180);

/// <span style="display: inline-block; background-color:#FF1493; width:20px; height:20px;"></span> Deep Pink, `#FF1493`, `rgb(255, 20, 147)`.
pub const DEEP_PINK: Rgba = rgb!(255, 20, 147);

/// <span style="display: inline-block; background-color:#DB7093; width:20px; height:20px;"></span> Pale Violet Red, `#DB7093`, `rgb(219, 112, 147)`.
pub const PALE_VIOLET_RED: Rgba = rgb!(219, 112, 147);

/// <span style="display: inline-block; background-color:#C71585; width:20px; height:20px;"></span> Medium Violet Red, `#C71585`, `rgb(199, 21, 133)`.
pub const MEDIUM_VIOLET_RED: Rgba = rgb!(199, 21, 133);

/// <span style="display: inline-block; background-color:#FFA07A; width:20px; height:20px;"></span> Light Salmon, `#FFA07A`, `rgb(255, 160, 122)`.
pub const LIGHT_SALMON: Rgba = rgb!(255, 160, 122);

/// <span style="display: inline-block; background-color:#FA8072; width:20px; height:20px;"></span> Salmon, `#FA8072`, `rgb(250, 128, 114)`.
pub const SALMON: Rgba = rgb!(250, 128, 114);

/// <span style="display: inline-block; background-color:#E9967A; width:20px; height:20px;"></span> Dark Salmon, `#E9967A`, `rgb(233, 150, 122)`.
pub const DARK_SALMON: Rgba = rgb!(233, 150, 122);

/// <span style="display: inline-block; background-color:#F08080; width:20px; height:20px;"></span> Light Coral, `#F08080`, `rgb(240, 128, 128)`.
pub const LIGHT_CORAL: Rgba = rgb!(240, 128, 128);

/// <span style="display: inline-block; background-color:#CD5C5C; width:20px; height:20px;"></span> Indian Red, `#CD5C5C`, `rgb(205, 92, 92)`.
pub const INDIAN_RED: Rgba = rgb!(205, 92, 92);

/// <span style="display: inline-block; background-color:#DC143C; width:20px; height:20px;"></span> Crimson, `#DC143C`, `rgb(220, 20, 60)`.
pub const CRIMSON: Rgba = rgb!(220, 20, 60);

/// <span style="display: inline-block; background-color:#B22222; width:20px; height:20px;"></span> Fire Brick, `#B22222`, `rgb(178, 34, 34)`.
pub const FIRE_BRICK: Rgba = rgb!(178, 34, 34);

/// <span style="display: inline-block; background-color:#8B0000; width:20px; height:20px;"></span> Dark Red, `#8B0000`, `rgb(139, 0, 0)`.
pub const DARK_RED: Rgba = rgb!(139, 0, 0);

/// <span style="display: inline-block; background-color:#FF0000; width:20px; height:20px;"></span> Red, `#FF0000`, `rgb(255, 0, 0)`.
pub const RED: Rgba = rgb!(255, 0, 0);

/// <span style="display: inline-block; background-color:#FF4500; width:20px; height:20px;"></span> Orange Red, `#FF4500`, `rgb(255, 69, 0)`.
pub const ORANGE_RED: Rgba = rgb!(255, 69, 0);

/// <span style="display: inline-block; background-color:#FF6347; width:20px; height:20px;"></span> Tomato, `#FF6347`, `rgb(255, 99, 71)`.
pub const TOMATO: Rgba = rgb!(255, 99, 71);

/// <span style="display: inline-block; background-color:#FF7F50; width:20px; height:20px;"></span> Coral, `#FF7F50`, `rgb(255, 127, 80)`.
pub const CORAL: Rgba = rgb!(255, 127, 80);

/// <span style="display: inline-block; background-color:#FF8C00; width:20px; height:20px;"></span> Dark Orange, `#FF8C00`, `rgb(255, 140, 0)`.
pub const DARK_ORANGE: Rgba = rgb!(255, 140, 0);

/// <span style="display: inline-block; background-color:#FFA500; width:20px; height:20px;"></span> Orange, `#FFA500`, `rgb(255, 165, 0)`.
pub const ORANGE: Rgba = rgb!(255, 165, 0);

/// <span style="display: inline-block; background-color:#FFFF00; width:20px; height:20px;"></span> Yellow, `#FFFF00`, `rgb(255, 255, 0)`.
pub const YELLOW: Rgba = rgb!(255, 255, 0);

/// <span style="display: inline-block; background-color:#FFFFE0; width:20px; height:20px;"></span> Light Yellow, `#FFFFE0`, `rgb(255, 255, 224)`.
pub const LIGHT_YELLOW: Rgba = rgb!(255, 255, 224);

/// <span style="display: inline-block; background-color:#FFFACD; width:20px; height:20px;"></span> Lemon Chiffon, `#FFFACD`, `rgb(255, 250, 205)`.
pub const LEMON_CHIFFON: Rgba = rgb!(255, 250, 205);

/// <span style="display: inline-block; background-color:#FAFAD2; width:20px; height:20px;"></span> Light Goldenrod Yellow, `#FAFAD2`, `rgb(250, 250, 210)`.
pub const LIGHT_GOLDENROD_YELLOW: Rgba = rgb!(250, 250, 210);

/// <span style="display: inline-block; background-color:#FFEFD5; width:20px; height:20px;"></span> Papaya Whip, `#FFEFD5`, `rgb(255, 239, 213)`.
pub const PAPAYA_WHIP: Rgba = rgb!(255, 239, 213);

/// <span style="display: inline-block; background-color:#FFE4B5; width:20px; height:20px;"></span> Moccasin, `#FFE4B5`, `rgb(255, 228, 181)`.
pub const MOCCASIN: Rgba = rgb!(255, 228, 181);

/// <span style="display: inline-block; background-color:#FFDAB9; width:20px; height:20px;"></span> Peach Puff, `#FFDAB9`, `rgb(255, 218, 185)`.
pub const PEACH_PUFF: Rgba = rgb!(255, 218, 185);

/// <span style="display: inline-block; background-color:#EEE8AA; width:20px; height:20px;"></span> Pale Goldenrod, `#EEE8AA`, `rgb(238, 232, 170)`.
pub const PALE_GOLDENROD: Rgba = rgb!(238, 232, 170);

/// <span style="display: inline-block; background-color:#F0E68C; width:20px; height:20px;"></span> Khaki, `#F0E68C`, `rgb(240, 230, 140)`.
pub const KHAKI: Rgba = rgb!(240, 230, 140);

/// <span style="display: inline-block; background-color:#BDB76B; width:20px; height:20px;"></span> Dark Khaki, `#BDB76B`, `rgb(189, 183, 107)`.
pub const DARK_KHAKI: Rgba = rgb!(189, 183, 107);

/// <span style="display: inline-block; background-color:#FFD700; width:20px; height:20px;"></span> Gold, `#FFD700`, `rgb(255, 215, 0)`.
pub const GOLD: Rgba = rgb!(255, 215, 0);

/// <span style="display: inline-block; background-color:#FFF8DC; width:20px; height:20px;"></span> Cornsilk, `#FFF8DC`, `rgb(255, 248, 220)`.
pub const CORNSILK: Rgba = rgb!(255, 248, 220);

/// <span style="display: inline-block; background-color:#FFEBCD; width:20px; height:20px;"></span> Blanched Almond, `#FFEBCD`, `rgb(255, 235, 205)`.
pub const BLANCHED_ALMOND: Rgba = rgb!(255, 235, 205);

/// <span style="display: inline-block; background-color:#FFE4C4; width:20px; height:20px;"></span> Bisque, `#FFE4C4`, `rgb(255, 228, 196)`.
pub const BISQUE: Rgba = rgb!(255, 228, 196);

/// <span style="display: inline-block; background-color:#FFDEAD; width:20px; height:20px;"></span> Navajo White, `#FFDEAD`, `rgb(255, 222, 173)`.
pub const NAVAJO_WHITE: Rgba = rgb!(255, 222, 173);

/// <span style="display: inline-block; background-color:#F5DEB3; width:20px; height:20px;"></span> Wheat, `#F5DEB3`, `rgb(245, 222, 179)`.
pub const WHEAT: Rgba = rgb!(245, 222, 179);

/// <span style="display: inline-block; background-color:#DEB887; width:20px; height:20px;"></span> Burly Wood, `#DEB887`, `rgb(222, 184, 135)`.
pub const BURLY_WOOD: Rgba = rgb!(222, 184, 135);

/// <span style="display: inline-block; background-color:#D2B48C; width:20px; height:20px;"></span> Tan, `#D2B48C`, `rgb(210, 180, 140)`.
pub const TAN: Rgba = rgb!(210, 180, 140);

/// <span style="display: inline-block; background-color:#BC8F8F; width:20px; height:20px;"></span> Rosy Brown, `#BC8F8F`, `rgb(188, 143, 143)`.
pub const ROSY_BROWN: Rgba = rgb!(188, 143, 143);

/// <span style="display: inline-block; background-color:#F4A460; width:20px; height:20px;"></span> Sandy Brown, `#F4A460`, `rgb(244, 164, 96)`.
pub const SANDY_BROWN: Rgba = rgb!(244, 164, 96);

/// <span style="display: inline-block; background-color:#DAA520; width:20px; height:20px;"></span> Goldenrod, `#DAA520`, `rgb(218, 165, 32)`.
pub const GOLDENROD: Rgba = rgb!(218, 165, 32);

/// <span style="display: inline-block; background-color:#B8860B; width:20px; height:20px;"></span> Dark Goldenrod, `#B8860B`, `rgb(184, 134, 11)`.
pub const DARK_GOLDENROD: Rgba = rgb!(184, 134, 11);

/// <span style="display: inline-block; background-color:#CD853F; width:20px; height:20px;"></span> Peru, `#CD853F`, `rgb(205, 133, 63)`.
pub const PERU: Rgba = rgb!(205, 133, 63);

/// <span style="display: inline-block; background-color:#D2691E; width:20px; height:20px;"></span> Chocolate, `#D2691E`, `rgb(210, 105, 30)`.
pub const CHOCOLATE: Rgba = rgb!(210, 105, 30);

/// <span style="display: inline-block; background-color:#8B4513; width:20px; height:20px;"></span> Saddle Brown, `#8B4513`, `rgb(139, 69, 19)`.
pub const SADDLE_BROWN: Rgba = rgb!(139, 69, 19);

/// <span style="display: inline-block; background-color:#A0522D; width:20px; height:20px;"></span> Sienna, `#A0522D`, `rgb(160, 82, 45)`.
pub const SIENNA: Rgba = rgb!(160, 82, 45);

/// <span style="display: inline-block; background-color:#A52A2A; width:20px; height:20px;"></span> Brown, `#A52A2A`, `rgb(165, 42, 42)`.
pub const BROWN: Rgba = rgb!(165, 42, 42);

/// <span style="display: inline-block; background-color:#800000; width:20px; height:20px;"></span> Maroon, `#800000`, `rgb(128, 0, 0)`.
pub const MAROON: Rgba = rgb!(128, 0, 0);

/// <span style="display: inline-block; background-color:#556B2F; width:20px; height:20px;"></span> Dark Olive Green, `#556B2F`, `rgb(85, 107, 47)`.
pub const DARK_OLIVE_GREEN: Rgba = rgb!(85, 107, 47);

/// <span style="display: inline-block; background-color:#808000; width:20px; height:20px;"></span> Olive, `#808000`, `rgb(128, 128, 0)`.
pub const OLIVE: Rgba = rgb!(128, 128, 0);

/// <span style="display: inline-block; background-color:#6B8E23; width:20px; height:20px;"></span> Olive Drab, `#6B8E23`, `rgb(107, 142, 35)`.
pub const OLIVE_DRAB: Rgba = rgb!(107, 142, 35);

/// <span style="display: inline-block; background-color:#9ACD32; width:20px; height:20px;"></span> Yellow Green, `#9ACD32`, `rgb(154, 205, 50)`.
pub const YELLOW_GREEN: Rgba = rgb!(154, 205, 50);

/// <span style="display: inline-block; background-color:#32CD32; width:20px; height:20px;"></span> Lime Green, `#32CD32`, `rgb(50, 205, 50)`.
pub const LIME_GREEN: Rgba = rgb!(50, 205, 50);

/// <span style="display: inline-block; background-color:#00FF00; width:20px; height:20px;"></span> Lime, `#00FF00`, `rgb(0, 255, 0)`.
pub const LIME: Rgba = rgb!(0, 255, 0);

/// <span style="display: inline-block; background-color:#7CFC00; width:20px; height:20px;"></span> Lawn Green, `#7CFC00`, `rgb(124, 252, 0)`.
pub const LAWN_GREEN: Rgba = rgb!(124, 252, 0);

/// <span style="display: inline-block; background-color:#7FFF00; width:20px; height:20px;"></span> Chartreuse, `#7FFF00`, `rgb(127, 255, 0)`.
pub const CHARTREUSE: Rgba = rgb!(127, 255, 0);

/// <span style="display: inline-block; background-color:#ADFF2F; width:20px; height:20px;"></span> Green Yellow, `#ADFF2F`, `rgb(173, 255, 47)`.
pub const GREEN_YELLOW: Rgba = rgb!(173, 255, 47);

/// <span style="display: inline-block; background-color:#00FF7F; width:20px; height:20px;"></span> Spring Green, `#00FF7F`, `rgb(0, 255, 127)`.
pub const SPRING_GREEN: Rgba = rgb!(0, 255, 127);

/// <span style="display: inline-block; background-color:#00FA9A; width:20px; height:20px;"></span> Medium Spring Green, `#00FA9A`, `rgb(0, 250, 154)`.
pub const MEDIUM_SPRING_GREEN: Rgba = rgb!(0, 250, 154);

/// <span style="display: inline-block; background-color:#90EE90; width:20px; height:20px;"></span> Light Green, `#90EE90`, `rgb(144, 238, 144)`.
pub const LIGHT_GREEN: Rgba = rgb!(144, 238, 144);

/// <span style="display: inline-block; background-color:#98FB98; width:20px; height:20px;"></span> Pale Green, `#98FB98`, `rgb(152, 251, 152)`.
pub const PALE_GREEN: Rgba = rgb!(152, 251, 152);

/// <span style="display: inline-block; background-color:#8FBC8F; width:20px; height:20px;"></span> Dark Sea Green, `#8FBC8F`, `rgb(143, 188, 143)`.
pub const DARK_SEA_GREEN: Rgba = rgb!(143, 188, 143);

/// <span style="display: inline-block; background-color:#3CB371; width:20px; height:20px;"></span> Medium Sea Green, `#3CB371`, `rgb(60, 179, 113)`.
pub const MEDIUM_SEA_GREEN: Rgba = rgb!(60, 179, 113);

/// <span style="display: inline-block; background-color:#2E8B57; width:20px; height:20px;"></span> Sea Green, `#2E8B57`, `rgb(46, 139, 87)`.
pub const SEA_GREEN: Rgba = rgb!(46, 139, 87);

/// <span style="display: inline-block; background-color:#228B22; width:20px; height:20px;"></span> Forest Green, `#228B22`, `rgb(34, 139, 34)`.
pub const FOREST_GREEN: Rgba = rgb!(34, 139, 34);

/// <span style="display: inline-block; background-color:#008000; width:20px; height:20px;"></span> Green, `#008000`, `rgb(0, 128, 0)`.
pub const GREEN: Rgba = rgb!(0, 128, 0);

/// <span style="display: inline-block; background-color:#006400; width:20px; height:20px;"></span> Dark Green, `#006400`, `rgb(0, 100, 0)`.
pub const DARK_GREEN: Rgba = rgb!(0, 100, 0);

/// <span style="display: inline-block; background-color:#66CDAA; width:20px; height:20px;"></span> Medium Aquamarine, `#66CDAA`, `rgb(102, 205, 170)`.
pub const MEDIUM_AQUAMARINE: Rgba = rgb!(102, 205, 170);

/// <span style="display: inline-block; background-color:#00FFFF; width:20px; height:20px;"></span> Aqua, `#00FFFF`, `rgb(0, 255, 255)`.
pub const AQUA: Rgba = rgb!(0, 255, 255);

/// <span style="display: inline-block; background-color:#00FFFF; width:20px; height:20px;"></span> Cyan, `#00FFFF`, `rgb(0, 255, 255)`.
pub const CYAN: Rgba = rgb!(0, 255, 255);

/// <span style="display: inline-block; background-color:#E0FFFF; width:20px; height:20px;"></span> Light Cyan, `#E0FFFF`, `rgb(224, 255, 255)`.
pub const LIGHT_CYAN: Rgba = rgb!(224, 255, 255);

/// <span style="display: inline-block; background-color:#AFEEEE; width:20px; height:20px;"></span> Pale Turquoise, `#AFEEEE`, `rgb(175, 238, 238)`.
pub const PALE_TURQUOISE: Rgba = rgb!(175, 238, 238);

/// <span style="display: inline-block; background-color:#7FFFD4; width:20px; height:20px;"></span> Aquamarine, `#7FFFD4`, `rgb(127, 255, 212)`.
pub const AQUAMARINE: Rgba = rgb!(127, 255, 212);

/// <span style="display: inline-block; background-color:#40E0D0; width:20px; height:20px;"></span> Turquoise, `#40E0D0`, `rgb(64, 224, 208)`.
pub const TURQUOISE: Rgba = rgb!(64, 224, 208);

/// <span style="display: inline-block; background-color:#48D1CC; width:20px; height:20px;"></span> Medium Turquoise, `#48D1CC`, `rgb(72, 209, 204)`.
pub const MEDIUM_TURQUOISE: Rgba = rgb!(72, 209, 204);

/// <span style="display: inline-block; background-color:#00CED1; width:20px; height:20px;"></span> Dark Turquoise, `#00CED1`, `rgb(0, 206, 209)`.
pub const DARK_TURQUOISE: Rgba = rgb!(0, 206, 209);

/// <span style="display: inline-block; background-color:#20B2AA; width:20px; height:20px;"></span> Light Sea Green, `#20B2AA`, `rgb(32, 178, 170)`.
pub const LIGHT_SEA_GREEN: Rgba = rgb!(32, 178, 170);

/// <span style="display: inline-block; background-color:#5F9EA0; width:20px; height:20px;"></span> Cadet Blue, `#5F9EA0`, `rgb(95, 158, 160)`.
pub const CADET_BLUE: Rgba = rgb!(95, 158, 160);

/// <span style="display: inline-block; background-color:#008B8B; width:20px; height:20px;"></span> Dark Cyan, `#008B8B`, `rgb(0, 139, 139)`.
pub const DARK_CYAN: Rgba = rgb!(0, 139, 139);

/// <span style="display: inline-block; background-color:#008080; width:20px; height:20px;"></span> Teal, `#008080`, `rgb(0, 128, 128)`.
pub const TEAL: Rgba = rgb!(0, 128, 128);

/// <span style="display: inline-block; background-color:#B0C4DE; width:20px; height:20px;"></span> Light Steel Blue, `#B0C4DE`, `rgb(176, 196, 222)`.
pub const LIGHT_STEEL_BLUE: Rgba = rgb!(176, 196, 222);

/// <span style="display: inline-block; background-color:#B0E0E6; width:20px; height:20px;"></span> Powder Blue, `#B0E0E6`, `rgb(176, 224, 230)`.
pub const POWDER_BLUE: Rgba = rgb!(176, 224, 230);

/// <span style="display: inline-block; background-color:#ADD8E6; width:20px; height:20px;"></span> Light Blue, `#ADD8E6`, `rgb(173, 216, 230)`.
pub const LIGHT_BLUE: Rgba = rgb!(173, 216, 230);

/// <span style="display: inline-block; background-color:#87CEEB; width:20px; height:20px;"></span> Sky Blue, `#87CEEB`, `rgb(135, 206, 235)`.
pub const SKY_BLUE: Rgba = rgb!(135, 206, 235);

/// <span style="display: inline-block; background-color:#87CEFA; width:20px; height:20px;"></span> Light Sky Blue, `#87CEFA`, `rgb(135, 206, 250)`.
pub const LIGHT_SKY_BLUE: Rgba = rgb!(135, 206, 250);

/// <span style="display: inline-block; background-color:#00BFFF; width:20px; height:20px;"></span> Deep Sky Blue, `#00BFFF`, `rgb(0, 191, 255)`.
pub const DEEP_SKY_BLUE: Rgba = rgb!(0, 191, 255);

/// <span style="display: inline-block; background-color:#1E90FF; width:20px; height:20px;"></span> Dodger Blue, `#1E90FF`, `rgb(30, 144, 255)`.
pub const DODGER_BLUE: Rgba = rgb!(30, 144, 255);

/// <span style="display: inline-block; background-color:#6495ED; width:20px; height:20px;"></span> Cornflower Blue, `#6495ED`, `rgb(100, 149, 237)`.
pub const CORNFLOWER_BLUE: Rgba = rgb!(100, 149, 237);

/// <span style="display: inline-block; background-color:#4682B4; width:20px; height:20px;"></span> Steel Blue, `#4682B4`, `rgb(70, 130, 180)`.
pub const STEEL_BLUE: Rgba = rgb!(70, 130, 180);

/// <span style="display: inline-block; background-color:#4169E1; width:20px; height:20px;"></span> Royal Blue, `#4169E1`, `rgb(65, 105, 225)`.
pub const ROYAL_BLUE: Rgba = rgb!(65, 105, 225);

/// <span style="display: inline-block; background-color:#0000FF; width:20px; height:20px;"></span> Blue, `#0000FF`, `rgb(0, 0, 255)`.
pub const BLUE: Rgba = rgb!(0, 0, 255);

/// <span style="display: inline-block; background-color:#0000CD; width:20px; height:20px;"></span> Medium Blue, `#0000CD`, `rgb(0, 0, 205)`.
pub const MEDIUM_BLUE: Rgba = rgb!(0, 0, 205);

/// <span style="display: inline-block; background-color:#00008B; width:20px; height:20px;"></span> Dark Blue, `#00008B`, `rgb(0, 0, 139)`.
pub const DARK_BLUE: Rgba = rgb!(0, 0, 139);

/// <span style="display: inline-block; background-color:#000080; width:20px; height:20px;"></span> Navy, `#000080`, `rgb(0, 0, 128)`.
pub const NAVY: Rgba = rgb!(0, 0, 128);

/// <span style="display: inline-block; background-color:#191970; width:20px; height:20px;"></span> Midnight Blue, `#191970`, `rgb(25, 25, 112)`.
pub const MIDNIGHT_BLUE: Rgba = rgb!(25, 25, 112);

/// <span style="display: inline-block; background-color:#FFFFFF; width:20px; height:20px;"></span> White, `#FFFFFF`, `rgb(255, 255, 255)`.
pub const WHITE: Rgba = rgb!(255, 255, 255);

/// <span style="display: inline-block; background-color:#FFFAFA; width:20px; height:20px;"></span> Snow, `#FFFAFA`, `rgb(255, 250, 250)`.
pub const SNOW: Rgba = rgb!(255, 250, 250);

/// <span style="display: inline-block; background-color:#F0FFF0; width:20px; height:20px;"></span> Honeydew, `#F0FFF0`, `rgb(240, 255, 240)`.
pub const HONEYDEW: Rgba = rgb!(240, 255, 240);

/// <span style="display: inline-block; background-color:#F5FFFA; width:20px; height:20px;"></span> Mint Cream, `#F5FFFA`, `rgb(245, 255, 250)`.
pub const MINT_CREAM: Rgba = rgb!(245, 255, 250);

/// <span style="display: inline-block; background-color:#F0FFFF; width:20px; height:20px;"></span> Azure, `#F0FFFF`, `rgb(240, 255, 255)`.
pub const AZURE: Rgba = rgb!(240, 255, 255);

/// <span style="display: inline-block; background-color:#F0F8FF; width:20px; height:20px;"></span> Alice Blue, `#F0F8FF`, `rgb(240, 248, 255)`.
pub const ALICE_BLUE: Rgba = rgb!(240, 248, 255);

/// <span style="display: inline-block; background-color:#F8F8FF; width:20px; height:20px;"></span> Ghost White, `#F8F8FF`, `rgb(248, 248, 255)`.
pub const GHOST_WHITE: Rgba = rgb!(248, 248, 255);

/// <span style="display: inline-block; background-color:#F5F5F5; width:20px; height:20px;"></span> White Smoke, `#F5F5F5`, `rgb(245, 245, 245)`.
pub const WHITE_SMOKE: Rgba = rgb!(245, 245, 245);

/// <span style="display: inline-block; background-color:#FFF5EE; width:20px; height:20px;"></span> Seashell, `#FFF5EE`, `rgb(255, 245, 238)`.
pub const SEASHELL: Rgba = rgb!(255, 245, 238);

/// <span style="display: inline-block; background-color:#F5F5DC; width:20px; height:20px;"></span> Beige, `#F5F5DC`, `rgb(245, 245, 220)`.
pub const BEIGE: Rgba = rgb!(245, 245, 220);

/// <span style="display: inline-block; background-color:#FDF5E6; width:20px; height:20px;"></span> Old Lace, `#FDF5E6`, `rgb(253, 245, 230)`.
pub const OLD_LACE: Rgba = rgb!(253, 245, 230);

/// <span style="display: inline-block; background-color:#FFFAF0; width:20px; height:20px;"></span> Floral White, `#FFFAF0`, `rgb(255, 250, 240)`.
pub const FLORAL_WHITE: Rgba = rgb!(255, 250, 240);

/// <span style="display: inline-block; background-color:#FFFFF0; width:20px; height:20px;"></span> Ivory, `#FFFFF0`, `rgb(255, 255, 240)`.
pub const IVORY: Rgba = rgb!(255, 255, 240);

/// <span style="display: inline-block; background-color:#FAEBD7; width:20px; height:20px;"></span> Antique White, `#FAEBD7`, `rgb(250, 235, 215)`.
pub const ANTIQUE_WHITE: Rgba = rgb!(250, 235, 215);

/// <span style="display: inline-block; background-color:#FAF0E6; width:20px; height:20px;"></span> Linen, `#FAF0E6`, `rgb(250, 240, 230)`.
pub const LINEN: Rgba = rgb!(250, 240, 230);

/// <span style="display: inline-block; background-color:#FFF0F5; width:20px; height:20px;"></span> Lavender Blush, `#FFF0F5`, `rgb(255, 240, 245)`.
pub const LAVENDER_BLUSH: Rgba = rgb!(255, 240, 245);

/// <span style="display: inline-block; background-color:#FFE4E1; width:20px; height:20px;"></span> Misty Rose, `#FFE4E1`, `rgb(255, 228, 225)`.
pub const MISTY_ROSE: Rgba = rgb!(255, 228, 225);

/// <span style="display: inline-block; background-color:#DCDCDC; width:20px; height:20px;"></span> Gainsboro, `#DCDCDC`, `rgb(220, 220, 220)`.
pub const GAINSBORO: Rgba = rgb!(220, 220, 220);

/// <span style="display: inline-block; background-color:#D3D3D3; width:20px; height:20px;"></span> Light Gray, `#D3D3D3`, `rgb(211, 211, 211)`.
pub const LIGHT_GRAY: Rgba = rgb!(211, 211, 211);

/// <span style="display: inline-block; background-color:#C0C0C0; width:20px; height:20px;"></span> Silver, `#C0C0C0`, `rgb(192, 192, 192)`.
pub const SILVER: Rgba = rgb!(192, 192, 192);

/// <span style="display: inline-block; background-color:#A9A9A9; width:20px; height:20px;"></span> Dark Gray, `#A9A9A9`, `rgb(169, 169, 169)`.
pub const DARK_GRAY: Rgba = rgb!(169, 169, 169);

/// <span style="display: inline-block; background-color:#808080; width:20px; height:20px;"></span> Gray, `#808080`, `rgb(128, 128, 128)`.
pub const GRAY: Rgba = rgb!(128, 128, 128);

/// <span style="display: inline-block; background-color:#696969; width:20px; height:20px;"></span> Dim Gray, `#696969`, `rgb(105, 105, 105)`.
pub const DIM_GRAY: Rgba = rgb!(105, 105, 105);

/// <span style="display: inline-block; background-color:#778899; width:20px; height:20px;"></span> Light Slate Gray, `#778899`, `rgb(119, 136, 153)`.
pub const LIGHT_SLATE_GRAY: Rgba = rgb!(119, 136, 153);

/// <span style="display: inline-block; background-color:#708090; width:20px; height:20px;"></span> Slate Gray, `#708090`, `rgb(112, 128, 144)`.
pub const SLATE_GRAY: Rgba = rgb!(112, 128, 144);

/// <span style="display: inline-block; background-color:#2F4F4F; width:20px; height:20px;"></span> Dark Slate Gray, `#2F4F4F`, `rgb(47, 79, 79)`.
pub const DARK_SLATE_GRAY: Rgba = rgb!(47, 79, 79);

/// <span style="display: inline-block; background-color:#000000; width:20px; height:20px;"></span> Black, `#000000`, `rgb(0, 0, 0)`.
pub const BLACK: Rgba = rgb!(0, 0, 0);
