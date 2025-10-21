/*
Loaded data is !Send+!Sync so we probably don't need to cache it.
*/

use std::{fmt, mem::size_of};

use byteorder::{BigEndian, ByteOrder as _, ReadBytesExt};
use icu_properties::props::{self, BinaryProperty};
use zng_color::{ColorScheme, Rgba, rgba};
use zng_var::impl_from_and_into_var;
use zng_view_api::font::GlyphIndex;

pub(super) fn maybe_emoji(c: char) -> bool {
    props::Emoji::for_char(c)
}

pub(super) fn definitely_emoji(c: char) -> bool {
    props::EmojiPresentation::for_char(c) || is_modifier(c)
}

pub(super) fn is_modifier(c: char) -> bool {
    props::EmojiModifier::for_char(c)
}

pub(super) fn is_component(c: char) -> bool {
    props::EmojiComponent::for_char(c)
}

/*
https://learn.microsoft.com/en-us/typography/opentype/spec/otff
All OpenType fonts use Motorola-style byte ordering (Big Endian)

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
/// [`FontFace::color_palettes`]: crate::FontFace::color_palettes
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
    pub fn load(font: &ttf_parser::RawFace) -> std::io::Result<Self> {
        // ttf_parser::Face does not expose CPAL

        let table = match font.table(ttf_parser::Tag(CPAL)) {
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

        let color_record_indices = cursor.position();
        let mut colors = Vec::with_capacity(num_palettes as usize * num_palette_entries as usize);
        for i in 0..num_palettes {
            cursor.set_position(color_record_indices + i as u64 * size_of::<u16>() as u64);

            let color_record_index = cursor.read_u16::<BigEndian>()? as u64;
            let color_record_offset = color_records_array_offset + color_record_index * size_of::<(u8, u8, u8, u8)>() as u64;

            cursor.set_position(color_record_offset);
            for _ in 0..num_palette_entries {
                let b = cursor.read_u8()?;
                let g = cursor.read_u8()?;
                let r = cursor.read_u8()?;
                let a = cursor.read_u8()?;

                colors.push(rgba(r, g, b, a));
            }
        }
        println!("!!: COLORS {:?}", colors.len());

        let mut palette_types = vec![];

        if version >= 1 {
            cursor.set_position(color_record_indices + num_palettes as u64 * size_of::<u16>() as u64);

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

    /// Gets the requested palette or the first if it is not found.
    pub fn palette(&self, p: impl Into<FontColorPalette>) -> Option<ColorPalette<'_>> {
        let i = self.palette_i(p.into());
        self.palette_get(i.unwrap_or(0))
    }

    /// Gets the requested palette.
    pub fn palette_exact(&self, p: impl Into<FontColorPalette>) -> Option<ColorPalette<'_>> {
        let i = self.palette_i(p.into())?;
        self.palette_get(i)
    }

    fn palette_i(&self, p: FontColorPalette) -> Option<usize> {
        match p {
            FontColorPalette::Light => self
                .types
                .iter()
                .position(|p| p.contains(ColorPaletteType::USABLE_WITH_LIGHT_BACKGROUND)),
            FontColorPalette::Dark => self
                .types
                .iter()
                .position(|p| p.contains(ColorPaletteType::USABLE_WITH_DARK_BACKGROUND)),
            FontColorPalette::Index(i) => {
                if i < self.num_palette_entries {
                    Some(i as _)
                } else {
                    None
                }
            }
        }
    }

    fn palette_get(&self, i: usize) -> Option<ColorPalette<'_>> {
        let len = self.num_palette_entries as usize;
        let s = len * i;
        let e = s + len;

        self.colors.get(s..e).map(|c| ColorPalette {
            flags: self.types.get(i).copied().unwrap_or_else(ColorPaletteType::empty),
            colors: c,
        })
    }

    /// Iterate over color palettes.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = ColorPalette<'_>> {
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
#[non_exhaustive]
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
/// [`FontFace::color_glyphs`]: crate::FontFace::color_glyphs
#[derive(Clone, Copy)]
pub struct ColorGlyphs<'a> {
    table: &'a [u8],
    num_base_glyph_records: u16,
    base_glyph_records_offset: u32,
    layer_records_offset: u32,
}
impl ColorGlyphs<'static> {
    /// No color glyphs.
    pub fn empty() -> Self {
        Self {
            table: &[],
            num_base_glyph_records: 0,
            base_glyph_records_offset: 0,
            layer_records_offset: 0,
        }
    }

    /// New from font.
    ///
    /// Color glyphs are parsed on demand.
    pub fn new<'a>(font: ttf_parser::RawFace<'a>) -> ColorGlyphs<'a> {
        match Self::new_impl(font) {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("error parsing color glyphs, {e}");
                Self::empty()
            }
        }
    }
    fn new_impl<'a>(font: ttf_parser::RawFace<'a>) -> std::io::Result<ColorGlyphs<'a>> {
        let table = match font.table(ttf_parser::Tag(COLR)) {
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

        let mut cursor = std::io::Cursor::new(table);

        let _version = cursor.read_u16::<BigEndian>()?;
        let num_base_glyph_records = cursor.read_u16::<BigEndian>()?;
        let base_glyph_records_offset = cursor.read_u32::<BigEndian>()?;
        let layer_records_offset = cursor.read_u32::<BigEndian>()?;

        Ok(ColorGlyphs {
            table,
            num_base_glyph_records,
            base_glyph_records_offset,
            layer_records_offset,
        })
    }
}
impl<'a> ColorGlyphs<'a> {
    /// If the font does not have any colored glyphs.
    pub fn is_empty(&self) -> bool {
        self.num_base_glyph_records > 0
    }

    /// Number of base glyphs that have colored replacements.
    pub fn len(&self) -> u16 {
        self.num_base_glyph_records
    }

    /// Gets the color glyph layers that replace the `base_glyph` to render in color.
    ///
    /// The `base_glyph` is the glyph selected by the font during shaping.
    ///
    /// Returns a [`ColorGlyph`] that provides the colored glyphs from the back (first item) to the front (last item).
    /// Paired with each glyph is an index in the font's [`ColorPalette::colors`] or `None` if the base text color must be used.
    ///
    /// Returns ``None  if the `base_glyph` has no associated colored replacements.
    pub fn glyph(&self, base_glyph: GlyphIndex) -> Option<ColorGlyph<'a>> {
        if self.is_empty() {
            return None;
        }

        let (first_layer_index, num_layers) = self.find_base_glyph(base_glyph)?;

        let record_size = 4;
        let table = &self.table[self.layer_records_offset as usize + (first_layer_index as usize * record_size)..];
        Some(ColorGlyph { table, num_layers })
    }

    /// Returns (firstLayerIndex, numLayers)
    fn find_base_glyph(&self, base_glyph: GlyphIndex) -> Option<(u16, u16)> {
        /*
        https://learn.microsoft.com/en-us/typography/opentype/spec/colr#baseglyph-and-layer-records

        BaseGlyph record:

        Type   Name            Description
        uint16 glyphID         Glyph ID of the base glyph.
        uint16 firstLayerIndex Index (base 0) into the layerRecords array.
        uint16 numLayers       Number of color layers associated with this glyph.
        */

        let base_glyph: u16 = base_glyph.try_into().ok()?;

        let record_size = 6;
        let base = self.base_glyph_records_offset as usize;

        let mut left = 0;
        let mut right = self.num_base_glyph_records as isize - 1;

        while left <= right {
            let mid = (left + right) / 2;
            let pos = base + mid as usize * record_size;

            // Safety: ensure within bounds
            if pos + record_size > self.table.len() {
                return None;
            }

            let glyph_id = BigEndian::read_u16(&self.table[pos..pos + 2]);
            match glyph_id.cmp(&base_glyph) {
                std::cmp::Ordering::Equal => {
                    let first_layer_index = BigEndian::read_u16(&self.table[pos + 2..pos + 4]);
                    let num_layers = BigEndian::read_u16(&self.table[pos + 4..pos + 6]);
                    return if num_layers > 0 {
                        Some((first_layer_index, num_layers))
                    } else {
                        None
                    };
                }
                std::cmp::Ordering::Less => left = mid + 1,
                std::cmp::Ordering::Greater => right = mid - 1,
            }
        }

        None
    }
}

/// Color glyph layers.
///
/// See [`ColorGlyphs::glyph`] for more details.
#[derive(Clone, Copy)]
pub struct ColorGlyph<'a> {
    table: &'a [u8],
    num_layers: u16,
}
impl<'a> ColorGlyph<'a> {
    /// Number of layers.
    ///
    /// This is always a non zero value as the [`ColorGlyphs`] returns `None` if there are no colored glyph replacements.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u16 {
        self.num_layers
    }

    /// Get the layer.
    ///
    /// # Panics
    ///
    /// Panics if `layer` is out of bounds.
    pub fn index(&self, layer: u16) -> (GlyphIndex, Option<u16>) {
        /*
        Layer record:

        Type   Name 	    Description
        uint16 glyphID      Glyph ID of the glyph used for a given layer.
        uint16 paletteIndex Index (base 0) for a palette entry in the CPAL table.
        */
        let t = &self.table[layer as usize * 4..];
        let glyph_id = BigEndian::read_u16(t);
        let pallet_index = BigEndian::read_u16(&t[2..]);
        if pallet_index == 0xFFFF {
            (glyph_id as _, None)
        } else {
            (glyph_id as _, Some(pallet_index))
        }
    }

    /// Iterate over layers, back to front.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (GlyphIndex, Option<u16>)> + '_ {
        (0..self.num_layers).map(move |i| self.index(i))
    }
}

/// Color palette selector for colored fonts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FontColorPalette {
    /// Select first font palette tagged [`ColorPaletteType::USABLE_WITH_LIGHT_BACKGROUND`], or 0 if the
    /// font does not tag any palette or no match is found.
    Light,
    /// Select first font palette tagged [`ColorPaletteType::USABLE_WITH_DARK_BACKGROUND`], or 0 if the
    /// font does not tag any palette or no match is found.
    Dark,
    /// Select one of the font provided palette by index.
    ///
    /// The palette list of a font is available in [`FontFace::color_palettes`]. If the index
    /// is not found uses the first font palette.
    ///
    /// [`FontFace::color_palettes`]: crate::FontFace::color_palettes
    Index(u16),
}
impl fmt::Debug for FontColorPalette {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "FontColorPalette::")?;
        }
        match self {
            Self::Light => write!(f, "Light"),
            Self::Dark => write!(f, "Dark"),
            Self::Index(arg0) => f.debug_tuple("Index").field(arg0).finish(),
        }
    }
}
impl_from_and_into_var! {
    fn from(index: u16) -> FontColorPalette {
        FontColorPalette::Index(index)
    }

    fn from(color_scheme: ColorScheme) -> FontColorPalette {
        match color_scheme {
            ColorScheme::Light => FontColorPalette::Light,
            ColorScheme::Dark => FontColorPalette::Dark,
            _ => FontColorPalette::Light,
        }
    }
}
