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

/// Lavender (`#E6E6FA`)
///
/// `rgb(230, 230, 250)`
pub const LAVENDER: Rgba = rgb!(230, 230, 250);

/// Thistle (`#D8BFD8`)
///
/// `rgb(216, 191, 216)`
pub const THISTLE: Rgba = rgb!(216, 191, 216);

/// Plum (`#DDA0DD`)
///
/// `rgb(221, 160, 221)`
pub const PLUM: Rgba = rgb!(221, 160, 221);

/// Violet (`#EE82EE`)
///
/// `rgb(238, 130, 238)`
pub const VIOLET: Rgba = rgb!(238, 130, 238);

/// Orchid (`#DA70D6`)
///
/// `rgb(218, 112, 214)`
pub const ORCHID: Rgba = rgb!(218, 112, 214);

/// Fuchsia (`#FF00FF`)
///
/// `rgb(255, 0, 255)`
pub const FUCHSIA: Rgba = rgb!(255, 0, 255);

/// Magenta (`#FF00FF`)
///
/// `rgb(255, 0, 255)`
pub const MAGENTA: Rgba = rgb!(255, 0, 255);

/// Medium Orchid (`#BA55D3`)
///
/// `rgb(186, 85, 211)`
pub const MEDIUM_ORCHID: Rgba = rgb!(186, 85, 211);

/// Medium Purple (`#9370DB`)
///
/// `rgb(147, 112, 219)`
pub const MEDIUM_PURPLE: Rgba = rgb!(147, 112, 219);

/// Blue Violet (`#8A2BE2`)
///
/// `rgb(138, 43, 226)`
pub const BLUE_VIOLET: Rgba = rgb!(138, 43, 226);

/// Dark Violet (`#9400D3`)
///
/// `rgb(148, 0, 211)`
pub const DARK_VIOLET: Rgba = rgb!(148, 0, 211);

/// Dark Orchid (`#9932CC`)
///
/// `rgb(153, 50, 204)`
pub const DARK_ORCHID: Rgba = rgb!(153, 50, 204);

/// Dark Magenta (`#8B008B`)
///
/// `rgb(139, 0, 139)`
pub const DARK_MAGENTA: Rgba = rgb!(139, 0, 139);

/// Purple (`#800080`)
///
/// `rgb(128, 0, 128)`
pub const PURPLE: Rgba = rgb!(128, 0, 128);

/// Indigo (`#4B0082`)
///
/// `rgb(75, 0, 130)`
pub const INDIGO: Rgba = rgb!(75, 0, 130);

/// Dark Slate Blue (`#483D8B`)
///
/// `rgb(72, 61, 139)`
pub const DARK_SLATE_BLUE: Rgba = rgb!(72, 61, 139);

/// Slate Blue (`#6A5ACD`)
///
/// `rgb(106, 90, 205)`
pub const SLATE_BLUE: Rgba = rgb!(106, 90, 205);

/// Medium Slate Blue (`#7B68EE`)
///
/// `rgb(123, 104, 238)`
pub const MEDIUM_SLATE_BLUE: Rgba = rgb!(123, 104, 238);

/// Pink (`#FFC0CB`)
///
/// `rgb(255, 192, 203)`
pub const PINK: Rgba = rgb!(255, 192, 203);

/// Light Pink (`#FFB6C1`)
///
/// `rgb(255, 182, 193)`
pub const LIGHT_PINK: Rgba = rgb!(255, 182, 193);

/// Hot Pink (`#FF69B4`)
///
/// `rgb(255, 105, 180)`
pub const HOT_PINK: Rgba = rgb!(255, 105, 180);

/// Deep Pink (`#FF1493`)
///
/// `rgb(255, 20, 147)`
pub const DEEP_PINK: Rgba = rgb!(255, 20, 147);

/// Pale Violet Red (`#DB7093`)
///
/// `rgb(219, 112, 147)`
pub const PALE_VIOLET_RED: Rgba = rgb!(219, 112, 147);

/// Medium Violet Red (`#C71585`)
///
/// `rgb(199, 21, 133)`
pub const MEDIUM_VIOLET_RED: Rgba = rgb!(199, 21, 133);

/// Light Salmon (`#FFA07A`)
///
/// `rgb(255, 160, 122)`
pub const LIGHT_SALMON: Rgba = rgb!(255, 160, 122);

/// Salmon (`#FA8072`)
///
/// `rgb(250, 128, 114)`
pub const SALMON: Rgba = rgb!(250, 128, 114);

/// Dark Salmon (`#E9967A`)
///
/// `rgb(233, 150, 122)`
pub const DARK_SALMON: Rgba = rgb!(233, 150, 122);

/// Light Coral (`#F08080`)
///
/// `rgb(240, 128, 128)`
pub const LIGHT_CORAL: Rgba = rgb!(240, 128, 128);

/// Indian Red (`#CD5C5C`)
///
/// `rgb(205, 92, 92)`
pub const INDIAN_RED: Rgba = rgb!(205, 92, 92);

/// Crimson (`#DC143C`)
///
/// `rgb(220, 20, 60)`
pub const CRIMSON: Rgba = rgb!(220, 20, 60);

/// Fire Brick (`#B22222`)
///
/// `rgb(178, 34, 34)`
pub const FIRE_BRICK: Rgba = rgb!(178, 34, 34);

/// Dark Red (`#8B0000`)
///
/// `rgb(139, 0, 0)`
pub const DARK_RED: Rgba = rgb!(139, 0, 0);

/// Red (`#FF0000`)
///
/// `rgb(255, 0, 0)`
pub const RED: Rgba = rgb!(255, 0, 0);

/// Orange Red (`#FF4500`)
///
/// `rgb(255, 69, 0)`
pub const ORANGE_RED: Rgba = rgb!(255, 69, 0);

/// Tomato (`#FF6347`)
///
/// `rgb(255, 99, 71)`
pub const TOMATO: Rgba = rgb!(255, 99, 71);

/// Coral (`#FF7F50`)
///
/// `rgb(255, 127, 80)`
pub const CORAL: Rgba = rgb!(255, 127, 80);

/// Dark Orange (`#FF8C00`)
///
/// `rgb(255, 140, 0)`
pub const DARK_ORANGE: Rgba = rgb!(255, 140, 0);

/// Orange (`#FFA500`)
///
/// `rgb(255, 165, 0)`
pub const ORANGE: Rgba = rgb!(255, 165, 0);

/// Yellow (`#FFFF00`)
///
/// `rgb(255, 255, 0)`
pub const YELLOW: Rgba = rgb!(255, 255, 0);

/// Light Yellow (`#FFFFE0`)
///
/// `rgb(255, 255, 224)`
pub const LIGHT_YELLOW: Rgba = rgb!(255, 255, 224);

/// Lemon Chiffon (`#FFFACD`)
///
/// `rgb(255, 250, 205)`
pub const LEMON_CHIFFON: Rgba = rgb!(255, 250, 205);

/// Light Goldenrod Yellow (`#FAFAD2`)
///
/// `rgb(250, 250, 210)`
pub const LIGHT_GOLDENROD_YELLOW: Rgba = rgb!(250, 250, 210);

/// Papaya Whip (`#FFEFD5`)
///
/// `rgb(255, 239, 213)`
pub const PAPAYA_WHIP: Rgba = rgb!(255, 239, 213);

/// Moccasin (`#FFE4B5`)
///
/// `rgb(255, 228, 181)`
pub const MOCCASIN: Rgba = rgb!(255, 228, 181);

/// Peach Puff (`#FFDAB9`)
///
/// `rgb(255, 218, 185)`
pub const PEACH_PUFF: Rgba = rgb!(255, 218, 185);

/// Pale Goldenrod (`#EEE8AA`)
///
/// `rgb(238, 232, 170)`
pub const PALE_GOLDENROD: Rgba = rgb!(238, 232, 170);

/// Khaki (`#F0E68C`)
///
/// `rgb(240, 230, 140)`
pub const KHAKI: Rgba = rgb!(240, 230, 140);

/// Dark Khaki (`#BDB76B`)
///
/// `rgb(189, 183, 107)`
pub const DARK_KHAKI: Rgba = rgb!(189, 183, 107);

/// Gold (`#FFD700`)
///
/// `rgb(255, 215, 0)`
pub const GOLD: Rgba = rgb!(255, 215, 0);

/// Cornsilk (`#FFF8DC`)
///
/// `rgb(255, 248, 220)`
pub const CORNSILK: Rgba = rgb!(255, 248, 220);

/// Blanched Almond (`#FFEBCD`)
///
/// `rgb(255, 235, 205)`
pub const BLANCHED_ALMOND: Rgba = rgb!(255, 235, 205);

/// Bisque (`#FFE4C4`)
///
/// `rgb(255, 228, 196)`
pub const BISQUE: Rgba = rgb!(255, 228, 196);

/// Navajo White (`#FFDEAD`)
///
/// `rgb(255, 222, 173)`
pub const NAVAJO_WHITE: Rgba = rgb!(255, 222, 173);

/// Wheat (`#F5DEB3`)
///
/// `rgb(245, 222, 179)`
pub const WHEAT: Rgba = rgb!(245, 222, 179);

/// Burly Wood (`#DEB887`)
///
/// `rgb(222, 184, 135)`
pub const BURLY_WOOD: Rgba = rgb!(222, 184, 135);

/// Tan (`#D2B48C`)
///
/// `rgb(210, 180, 140)`
pub const TAN: Rgba = rgb!(210, 180, 140);

/// Rosy Brown (`#BC8F8F`)
///
/// `rgb(188, 143, 143)`
pub const ROSY_BROWN: Rgba = rgb!(188, 143, 143);

/// Sandy Brown (`#F4A460`)
///
/// `rgb(244, 164, 96)`
pub const SANDY_BROWN: Rgba = rgb!(244, 164, 96);

/// Goldenrod (`#DAA520`)
///
/// `rgb(218, 165, 32)`
pub const GOLDENROD: Rgba = rgb!(218, 165, 32);

/// Dark Goldenrod (`#B8860B`)
///
/// `rgb(184, 134, 11)`
pub const DARK_GOLDENROD: Rgba = rgb!(184, 134, 11);

/// Peru (`#CD853F`)
///
/// `rgb(205, 133, 63)`
pub const PERU: Rgba = rgb!(205, 133, 63);

/// Chocolate (`#D2691E`)
///
/// `rgb(210, 105, 30)`
pub const CHOCOLATE: Rgba = rgb!(210, 105, 30);

/// Saddle Brown (`#8B4513`)
///
/// `rgb(139, 69, 19)`
pub const SADDLE_BROWN: Rgba = rgb!(139, 69, 19);

/// Sienna (`#A0522D`)
///
/// `rgb(160, 82, 45)`
pub const SIENNA: Rgba = rgb!(160, 82, 45);

/// Brown (`#A52A2A`)
///
/// `rgb(165, 42, 42)`
pub const BROWN: Rgba = rgb!(165, 42, 42);

/// Maroon (`#800000`)
///
/// `rgb(128, 0, 0)`
pub const MAROON: Rgba = rgb!(128, 0, 0);

/// Dark Olive Green (`#556B2F`)
///
/// `rgb(85, 107, 47)`
pub const DARK_OLIVE_GREEN: Rgba = rgb!(85, 107, 47);

/// Olive (`#808000`)
///
/// `rgb(128, 128, 0)`
pub const OLIVE: Rgba = rgb!(128, 128, 0);

/// Olive Drab (`#6B8E23`)
///
/// `rgb(107, 142, 35)`
pub const OLIVE_DRAB: Rgba = rgb!(107, 142, 35);

/// Yellow Green (`#9ACD32`)
///
/// `rgb(154, 205, 50)`
pub const YELLOW_GREEN: Rgba = rgb!(154, 205, 50);

/// Lime Green (`#32CD32`)
///
/// `rgb(50, 205, 50)`
pub const LIME_GREEN: Rgba = rgb!(50, 205, 50);

/// Lime (`#00FF00`)
///
/// `rgb(0, 255, 0)`
pub const LIME: Rgba = rgb!(0, 255, 0);

/// Lawn Green (`#7CFC00`)
///
/// `rgb(124, 252, 0)`
pub const LAWN_GREEN: Rgba = rgb!(124, 252, 0);

/// Chartreuse (`#7FFF00`)
///
/// `rgb(127, 255, 0)`
pub const CHARTREUSE: Rgba = rgb!(127, 255, 0);

/// Green Yellow (`#ADFF2F`)
///
/// `rgb(173, 255, 47)`
pub const GREEN_YELLOW: Rgba = rgb!(173, 255, 47);

/// Spring Green (`#00FF7F`)
///
/// `rgb(0, 255, 127)`
pub const SPRING_GREEN: Rgba = rgb!(0, 255, 127);

/// Medium Spring Green (`#00FA9A`)
///
/// `rgb(0, 250, 154)`
pub const MEDIUM_SPRING_GREEN: Rgba = rgb!(0, 250, 154);

/// Light Green (`#90EE90`)
///
/// `rgb(144, 238, 144)`
pub const LIGHT_GREEN: Rgba = rgb!(144, 238, 144);

/// Pale Green (`#98FB98`)
///
/// `rgb(152, 251, 152)`
pub const PALE_GREEN: Rgba = rgb!(152, 251, 152);

/// Dark Sea Green (`#8FBC8F`)
///
/// `rgb(143, 188, 143)`
pub const DARK_SEA_GREEN: Rgba = rgb!(143, 188, 143);

/// Medium Sea Green (`#3CB371`)
///
/// `rgb(60, 179, 113)`
pub const MEDIUM_SEA_GREEN: Rgba = rgb!(60, 179, 113);

/// Sea Green (`#2E8B57`)
///
/// `rgb(46, 139, 87)`
pub const SEA_GREEN: Rgba = rgb!(46, 139, 87);

/// Forest Green (`#228B22`)
///
/// `rgb(34, 139, 34)`
pub const FOREST_GREEN: Rgba = rgb!(34, 139, 34);

/// Green (`#008000`)
///
/// `rgb(0, 128, 0)`
pub const GREEN: Rgba = rgb!(0, 128, 0);

/// Dark Green (`#006400`)
///
/// `rgb(0, 100, 0)`
pub const DARK_GREEN: Rgba = rgb!(0, 100, 0);

/// Medium Aquamarine (`#66CDAA`)
///
/// `rgb(102, 205, 170)`
pub const MEDIUM_AQUAMARINE: Rgba = rgb!(102, 205, 170);

/// Aqua (`#00FFFF`)
///
/// `rgb(0, 255, 255)`
pub const AQUA: Rgba = rgb!(0, 255, 255);

/// Cyan (`#00FFFF`)
///
/// `rgb(0, 255, 255)`
pub const CYAN: Rgba = rgb!(0, 255, 255);

/// Light Cyan (`#E0FFFF`)
///
/// `rgb(224, 255, 255)`
pub const LIGHT_CYAN: Rgba = rgb!(224, 255, 255);

/// Pale Turquoise (`#AFEEEE`)
///
/// `rgb(175, 238, 238)`
pub const PALE_TURQUOISE: Rgba = rgb!(175, 238, 238);

/// Aquamarine (`#7FFFD4`)
///
/// `rgb(127, 255, 212)`
pub const AQUAMARINE: Rgba = rgb!(127, 255, 212);

/// Turquoise (`#40E0D0`)
///
/// `rgb(64, 224, 208)`
pub const TURQUOISE: Rgba = rgb!(64, 224, 208);

/// Medium Turquoise (`#48D1CC`)
///
/// `rgb(72, 209, 204)`
pub const MEDIUM_TURQUOISE: Rgba = rgb!(72, 209, 204);

/// Dark Turquoise (`#00CED1`)
///
/// `rgb(0, 206, 209)`
pub const DARK_TURQUOISE: Rgba = rgb!(0, 206, 209);

/// Light Sea Green (`#20B2AA`)
///
/// `rgb(32, 178, 170)`
pub const LIGHT_SEA_GREEN: Rgba = rgb!(32, 178, 170);

/// Cadet Blue (`#5F9EA0`)
///
/// `rgb(95, 158, 160)`
pub const CADET_BLUE: Rgba = rgb!(95, 158, 160);

/// Dark Cyan (`#008B8B`)
///
/// `rgb(0, 139, 139)`
pub const DARK_CYAN: Rgba = rgb!(0, 139, 139);

/// Teal (`#008080`)
///
/// `rgb(0, 128, 128)`
pub const TEAL: Rgba = rgb!(0, 128, 128);

/// Light Steel Blue (`#B0C4DE`)
///
/// `rgb(176, 196, 222)`
pub const LIGHT_STEEL_BLUE: Rgba = rgb!(176, 196, 222);

/// Powder Blue (`#B0E0E6`)
///
/// `rgb(176, 224, 230)`
pub const POWDER_BLUE: Rgba = rgb!(176, 224, 230);

/// Light Blue (`#ADD8E6`)
///
/// `rgb(173, 216, 230)`
pub const LIGHT_BLUE: Rgba = rgb!(173, 216, 230);

/// Sky Blue (`#87CEEB`)
///
/// `rgb(135, 206, 235)`
pub const SKY_BLUE: Rgba = rgb!(135, 206, 235);

/// Light Sky Blue (`#87CEFA`)
///
/// `rgb(135, 206, 250)`
pub const LIGHT_SKY_BLUE: Rgba = rgb!(135, 206, 250);

/// Deep Sky Blue (`#00BFFF`)
///
/// `rgb(0, 191, 255)`
pub const DEEP_SKY_BLUE: Rgba = rgb!(0, 191, 255);

/// Dodger Blue (`#1E90FF`)
///
/// `rgb(30, 144, 255)`
pub const DODGER_BLUE: Rgba = rgb!(30, 144, 255);

/// Cornflower Blue (`#6495ED`)
///
/// `rgb(100, 149, 237)`
pub const CORNFLOWER_BLUE: Rgba = rgb!(100, 149, 237);

/// Steel Blue (`#4682B4`)
///
/// `rgb(70, 130, 180)`
pub const STEEL_BLUE: Rgba = rgb!(70, 130, 180);

/// Royal Blue (`#4169E1`)
///
/// `rgb(65, 105, 225)`
pub const ROYAL_BLUE: Rgba = rgb!(65, 105, 225);

/// Blue (`#0000FF`)
///
/// `rgb(0, 0, 255)`
pub const BLUE: Rgba = rgb!(0, 0, 255);

/// Medium Blue (`#0000CD`)
///
/// `rgb(0, 0, 205)`
pub const MEDIUM_BLUE: Rgba = rgb!(0, 0, 205);

/// Dark Blue (`#00008B`)
///
/// `rgb(0, 0, 139)`
pub const DARK_BLUE: Rgba = rgb!(0, 0, 139);

/// Navy (`#000080`)
///
/// `rgb(0, 0, 128)`
pub const NAVY: Rgba = rgb!(0, 0, 128);

/// Midnight Blue (`#191970`)
///
/// `rgb(25, 25, 112)`
pub const MIDNIGHT_BLUE: Rgba = rgb!(25, 25, 112);

/// White (`#FFFFFF`)
///
/// `rgb(255, 255, 255)`
pub const WHITE: Rgba = rgb!(255, 255, 255);

/// Snow (`#FFFAFA`)
///
/// `rgb(255, 250, 250)`
pub const SNOW: Rgba = rgb!(255, 250, 250);

/// Honeydew (`#F0FFF0`)
///
/// `rgb(240, 255, 240)`
pub const HONEYDEW: Rgba = rgb!(240, 255, 240);

/// Mint Cream (`#F5FFFA`)
///
/// `rgb(245, 255, 250)`
pub const MINT_CREAM: Rgba = rgb!(245, 255, 250);

/// Azure (`#F0FFFF`)
///
/// `rgb(240, 255, 255)`
pub const AZURE: Rgba = rgb!(240, 255, 255);

/// Alice Blue (`#F0F8FF`)
///
/// `rgb(240, 248, 255)`
pub const ALICE_BLUE: Rgba = rgb!(240, 248, 255);

/// Ghost White (`#F8F8FF`)
///
/// `rgb(248, 248, 255)`
pub const GHOST_WHITE: Rgba = rgb!(248, 248, 255);

/// White Smoke (`#F5F5F5`)
///
/// `rgb(245, 245, 245)`
pub const WHITE_SMOKE: Rgba = rgb!(245, 245, 245);

/// Seashell (`#FFF5EE`)
///
/// `rgb(255, 245, 238)`
pub const SEASHELL: Rgba = rgb!(255, 245, 238);

/// Beige (`#F5F5DC`)
///
/// `rgb(245, 245, 220)`
pub const BEIGE: Rgba = rgb!(245, 245, 220);

/// Old Lace (`#FDF5E6`)
///
/// `rgb(253, 245, 230)`
pub const OLD_LACE: Rgba = rgb!(253, 245, 230);

/// Floral White (`#FFFAF0`)
///
/// `rgb(255, 250, 240)`
pub const FLORAL_WHITE: Rgba = rgb!(255, 250, 240);

/// Ivory (`#FFFFF0`)
///
/// `rgb(255, 255, 240)`
pub const IVORY: Rgba = rgb!(255, 255, 240);

/// Antique White (`#FAEBD7`)
///
/// `rgb(250, 235, 215)`
pub const ANTIQUE_WHITE: Rgba = rgb!(250, 235, 215);

/// Linen (`#FAF0E6`)
///
/// `rgb(250, 240, 230)`
pub const LINEN: Rgba = rgb!(250, 240, 230);

/// Lavender Blush (`#FFF0F5`)
///
/// `rgb(255, 240, 245)`
pub const LAVENDER_BLUSH: Rgba = rgb!(255, 240, 245);

/// Misty Rose (`#FFE4E1`)
///
/// `rgb(255, 228, 225)`
pub const MISTY_ROSE: Rgba = rgb!(255, 228, 225);

/// Gainsboro (`#DCDCDC`)
///
/// `rgb(220, 220, 220)`
pub const GAINSBORO: Rgba = rgb!(220, 220, 220);

/// Light Gray (`#D3D3D3`)
///
/// `rgb(211, 211, 211)`
pub const LIGHT_GRAY: Rgba = rgb!(211, 211, 211);

/// Silver (`#C0C0C0`)
///
/// `rgb(192, 192, 192)`
pub const SILVER: Rgba = rgb!(192, 192, 192);

/// Dark Gray (`#A9A9A9`)
///
/// `rgb(169, 169, 169)`
pub const DARK_GRAY: Rgba = rgb!(169, 169, 169);

/// Gray (`#808080`)
///
/// `rgb(128, 128, 128)`
pub const GRAY: Rgba = rgb!(128, 128, 128);

/// Dim Gray (`#696969`)
///
/// `rgb(105, 105, 105)`
pub const DIM_GRAY: Rgba = rgb!(105, 105, 105);

/// Light Slate Gray (`#778899`)
///
/// `rgb(119, 136, 153)`
pub const LIGHT_SLATE_GRAY: Rgba = rgb!(119, 136, 153);

/// Slate Gray (`#708090`)
///
/// `rgb(112, 128, 144)`
pub const SLATE_GRAY: Rgba = rgb!(112, 128, 144);

/// Dark Slate Gray (`#2F4F4F`)
///
/// `rgb(47, 79, 79)`
pub const DARK_SLATE_GRAY: Rgba = rgb!(47, 79, 79);

/// Black (`#000000`)
///
/// `rgb(0, 0, 0)`
pub const BLACK: Rgba = rgb!(0, 0, 0);
