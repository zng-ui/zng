/*
Loaded data is !Send+!Sync so we probably don't need to cache it.

The "icu_testdata" includes the stuff we need, plus a lot of useless data, there is a complicated way to
optmize this, but they are about to release embedded data, so we wait.

see: see https://github.com/unicode-org/icu4x/issues/3529

 */

use std::mem::size_of;

use byteorder::{BigEndian, ReadBytesExt};
use icu_properties::sets;
use zero_ui_view_api::webrender_api::GlyphIndex;

use crate::color::Rgba;

pub(super) fn maybe_emoji(c: char) -> bool {
    sets::load_emoji(&icu_testdata::unstable()).unwrap().as_borrowed().contains(c)
}

pub(super) fn definitely_emoji(c: char) -> bool {
    sets::load_emoji_presentation(&icu_testdata::unstable())
        .unwrap()
        .as_borrowed()
        .contains(c)
        || is_modifier(c)
}

pub(super) fn is_modifier(c: char) -> bool {
    sets::load_emoji_modifier(&icu_testdata::unstable())
        .unwrap()
        .as_borrowed()
        .contains(c)
}

pub(super) fn is_component(c: char) -> bool {
    sets::load_emoji_component(&icu_testdata::unstable())
        .unwrap()
        .as_borrowed()
        .contains(c)
}

/*
https://learn.microsoft.com/en-us/typography/opentype/spec/otff
All OpenType fonts use Motorola-style byte ordering (Big Endian

Offset32 = uint32
 */

// OpenType is Big Endian, table IDs are their ASCII name (4 chars) as an `u32`.

/// Color Palette Table
const CPAL: u32 = u32::from_be_bytes(*b"CPAL");

/// Color Table.
const COLR: u32 = u32::from_be_bytes(*b"COLR");

/// CPAL table.
///
/// The palettes for a font are available in [`FontFace::color_palettes`].
///
/// [`FontFace::color_palettes`]: crate::text::FontFace::color_palettes
#[derive(Clone, Debug)]
pub struct ColorPalettes {
    num_palettes: u16,
    num_palette_entries: u16,
    colors: Box<[Rgba]>,
    types: Box<[ColorPaletteType]>,
}
impl Default for ColorPalettes {
    /// Empty.
    fn default() -> Self {
        Self::empty()
    }
}
impl ColorPalettes {
    /// No palettes.
    pub fn empty() -> Self {
        Self {
            num_palettes: 0,
            num_palette_entries: 0,
            colors: Box::new([]),
            types: Box::new([]),
        }
    }

    /// Load the table, if present in the font.
    pub fn load(ft: &font_kit::font::Font) -> std::io::Result<Self> {
        let table = match ft.load_font_table(CPAL) {
            Some(t) => t,
            None => return Ok(Self::empty()),
        };

        /*
        https://learn.microsoft.com/en-us/typography/opentype/spec/cpal
        CPAL version 0

        The CPAL header version 0 is organized as follows:
        Type 	 Name 	                        Description
        uint16 	 version 	                    Table version number (=0).
        uint16 	 numPaletteEntries 	            Number of palette entries in each palette.
        uint16 	 numPalettes 	                Number of palettes in the table.
        uint16 	 numColorRecords 	            Total number of color records, combined for all palettes.
        Offset32 colorRecordsArrayOffset 	    Offset from the beginning of CPAL table to the first ColorRecord.
        uint16 	 colorRecordIndices[numPalettes] Index of each palette’s first color record in the combined color record array.
         */

        let mut cursor = std::io::Cursor::new(&table);

        let version = cursor.read_u16::<BigEndian>()?;
        let num_palette_entries = cursor.read_u16::<BigEndian>()?;
        let num_palettes = cursor.read_u16::<BigEndian>()?;
        let _num_color_records = cursor.read_u16::<BigEndian>()?;
        let color_records_array_offset = cursor.read_u32::<BigEndian>()? as u64;

        let color_record_indicies = cursor.position();
        let mut colors = Vec::with_capacity(num_palettes as usize * num_palette_entries as usize);
        for i in 0..num_palettes {
            cursor.set_position(color_record_indicies + i as u64 * size_of::<u16>() as u64);

            let color_record_indice = color_records_array_offset + cursor.read_u16::<BigEndian>()? as u64;

            cursor.set_position(color_record_indice);
            for _ in 0..num_palette_entries {
                let b = cursor.read_u8()?;
                let g = cursor.read_u8()?;
                let r = cursor.read_u8()?;
                let a = cursor.read_u8()?;

                colors.push(crate::color::rgba(r, g, b, a));
            }
        }

        let mut palette_types = vec![];

        if version >= 1 {
            cursor.set_position(color_record_indicies + num_palettes as u64 * size_of::<u16>() as u64);

            /*
            CPAL version 1

            {version..colorRecordIndices[numPalettes]}

            Offset32 paletteTypesArrayOffset 	   Offset from the beginning of CPAL table to the Palette Types Array. Set to 0 if no array is provided.
            Offset32 paletteLabelsArrayOffset 	   Offset from the beginning of CPAL table to the Palette Labels Array. Set to 0 if no array is provided.
            Offset32 paletteEntryLabelsArrayOffset Offset from the beginning of CPAL table to the Palette Entry Labels Array. Set to 0 if no array is provided.
            */
            let palette_types_array_offset = cursor.read_u32::<BigEndian>()? as u64;
            let _palette_labels_array_offset = cursor.read_u32::<BigEndian>()? as u64;
            let _palette_entry_labels_array_offset = cursor.read_u32::<BigEndian>()? as u64;

            if palette_types_array_offset > 0 {
                palette_types.reserve(num_palettes as usize);

                cursor.set_position(palette_types_array_offset);
                for _ in 0..num_palettes {
                    let flags = cursor.read_u32::<BigEndian>()?;
                    let flags = ColorPaletteType::from_bits(flags).unwrap_or_else(ColorPaletteType::empty);
                    palette_types.push(flags);
                }
            }
        }

        Ok(Self {
            num_palettes,
            num_palette_entries,
            colors: colors.into_boxed_slice(),
            types: palette_types.into_boxed_slice(),
        })
    }

    /// Number of palettes.
    pub fn len(&self) -> usize {
        self.num_palettes as usize
    }

    /// If the font does not have any color palette.
    pub fn is_empty(&self) -> bool {
        self.num_palettes == 0
    }

    /// Gets the palette.
    ///
    /// All palettes have the same length.
    pub fn palette(&self, i: usize) -> Option<ColorPalette> {
        let len = self.num_palette_entries as usize;
        let s = len + i;
        let e = s + len;

        self.colors.get(s..e).map(|c| ColorPalette {
            flags: self.types.get(i).copied().unwrap_or_else(ColorPaletteType::empty),
            colors: c,
        })
    }

    /// Iterate over color palettes.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = ColorPalette> {
        self.colors
            .chunks_exact(self.num_palette_entries as _)
            .enumerate()
            .map(|(i, c)| ColorPalette {
                flags: self.types.get(i).copied().unwrap_or_else(ColorPaletteType::empty),
                colors: c,
            })
    }
}

bitflags! {
    /// Represents a color palette v1 flag.
    ///
    /// See [`ColorPalettes`] for more details.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ColorPaletteType: u32 {
        /// Palette is appropriate to use when displaying the font on a light background such as white.
        const USABLE_WITH_LIGHT_BACKGROUND = 0x0001;
        /// Palette is appropriate to use when displaying the font on a dark background such as black.
        const USABLE_WITH_DARK_BACKGROUND = 0x0002;
    }
}

/// Represents a color palette entry.
///
/// See [`ColorPalettes`] for more details.
pub struct ColorPalette<'a> {
    /// Palette v1 flags.
    pub flags: ColorPaletteType,
    /// Palette colors.
    pub colors: &'a [Rgba],
}

/// COLR table.
///
/// The color glyphs for a font are available in [`FontFace::color_glyphs`].
///
/// [`FontFace::color_glyphs`]: crate::text::FontFace::color_glyphs
#[derive(Clone, Debug)]
pub struct ColorGlyphs {
    base_glyph_records: Vec<BaseGlyphRecord>,
    layer_records: Vec<LayerRecord>,
}
impl ColorGlyphs {
    /// No color glyphs.
    pub fn empty() -> Self {
        Self {
            base_glyph_records: vec![],
            layer_records: vec![],
        }
    }

    /// Load the table, if present in the font.
    pub fn load(ft: &font_kit::font::Font) -> std::io::Result<Self> {
        let table = match ft.load_font_table(COLR) {
            Some(t) => t,
            None => return Ok(Self::empty()),
        };

        /*
        https://learn.microsoft.com/en-us/typography/opentype/spec/colr#colr-formats
        COLR version 0

        Type 	 Name 	                Description
        uint16 	 version 	            Table version number—set to 0.
        uint16   numBaseGlyphRecords 	Number of BaseGlyph records.
        Offset32 baseGlyphRecordsOffset	Offset to baseGlyphRecords array.
        Offset32 layerRecordsOffset 	Offset to layerRecords array.
        uint16 	 numLayerRecords 	    Number of Layer records.
        */

        let mut cursor = std::io::Cursor::new(&table);

        let _version = cursor.read_u16::<BigEndian>()?;
        let num_base_glyph_records = cursor.read_u16::<BigEndian>()?;
        let base_glyph_records_offset = cursor.read_u32::<BigEndian>()? as u64;
        let layer_records_offset = cursor.read_u32::<BigEndian>()? as u64;
        let num_layer_records = cursor.read_u16::<BigEndian>()?;

        let mut base_glyph_records = Vec::with_capacity(num_base_glyph_records as _);

        cursor.set_position(base_glyph_records_offset);
        for _ in 0..num_base_glyph_records {
            /*
            https://learn.microsoft.com/en-us/typography/opentype/spec/colr#baseglyph-and-layer-records

            BaseGlyph record:

            Type   Name            Description
            uint16 glyphID         Glyph ID of the base glyph.
            uint16 firstLayerIndex Index (base 0) into the layerRecords array.
            uint16 numLayers       Number of color layers associated with this glyph.
            */

            base_glyph_records.push(BaseGlyphRecord {
                glyph_id: cursor.read_u16::<BigEndian>()?,
                first_layer_index: cursor.read_u16::<BigEndian>()?,
                num_layers: cursor.read_u16::<BigEndian>()?,
            });
        }

        let mut layer_records = Vec::with_capacity(num_layer_records as _);
        cursor.set_position(layer_records_offset);
        for _ in 0..num_layer_records {
            /*
            Layer record:

            Type   Name 	    Description
            uint16 glyphID      Glyph ID of the glyph used for a given layer.
            uint16 paletteIndex Index (base 0) for a palette entry in the CPAL table.
            */

            layer_records.push(LayerRecord {
                glyph_id: cursor.read_u16::<BigEndian>()?,
                palette_index: cursor.read_u16::<BigEndian>()?,
            });
        }

        Ok(Self {
            base_glyph_records,
            layer_records,
        })
    }

    /// Iterate over color glyphs that replace the `base_glyph` to render in color.
    ///
    /// The `base_glyph` is the glyph selected by the font during shaping.
    ///
    /// Returns an iterator that *renders* an Emoji by overlaying colored glyphs, from the back (first item)
    /// to the front (last item). Paired with each glyph is an index in the font's [`ColorPalette::colors`] or
    /// `None` if the base text color must be used.
    ///
    /// Yields the `base_glyph` with `None` color if it the font does not provide colored replacements for it.
    pub fn glyphs(&self, base_glyph: GlyphIndex) -> impl Iterator<Item = (GlyphIndex, Option<usize>)> + '_ {
        let layers = match self.base_glyph_records.binary_search_by_key(&(base_glyph as u16), |e| e.glyph_id) {
            Ok(i) => {
                let rec = &self.base_glyph_records[i];

                let s = rec.first_layer_index as usize;
                let e = s + rec.num_layers as usize;
                &self.layer_records[s..e]
            }
            Err(_) => &[],
        };

        let iter = layers.iter().map(|l| (l.glyph_id(), l.palette_index()));
        let mut not_found = [(base_glyph, None)].into_iter();
        if !layers.is_empty() {
            not_found.next();
        }
        iter.chain(not_found)
    }

    /// Resolve [`glyphs`] and colors.
    ///
    /// [`glyphs`]: Self::glyphs
    pub fn glyph_rgba<'a>(
        &'a self,
        txt_color: Rgba,
        palette: &'a [Rgba],
        base_glyph: GlyphIndex,
    ) -> impl Iterator<Item = (GlyphIndex, Rgba)> + 'a {
        self.glyphs(base_glyph)
            .map(move |(g, c)| (g, c.and_then(|i| palette.get(i).copied()).unwrap_or(txt_color)))
    }

    /// If the font does not have any colored glyphs.
    pub fn is_empty(&self) -> bool {
        self.base_glyph_records.is_empty()
    }

    /// Number of base glyphs that have colored replacements.
    pub fn len(&self) -> usize {
        self.base_glyph_records.len()
    }
}

#[derive(Debug, Clone, Copy)]
struct BaseGlyphRecord {
    glyph_id: u16,
    first_layer_index: u16,
    num_layers: u16,
}

#[derive(Debug, Clone, Copy)]
struct LayerRecord {
    glyph_id: u16,
    palette_index: u16,
}
impl LayerRecord {
    fn glyph_id(&self) -> GlyphIndex {
        self.glyph_id as _
    }

    fn palette_index(&self) -> Option<usize> {
        if self.palette_index == 0xFFFF {
            None
        } else {
            Some(self.palette_index as _)
        }
    }
}
