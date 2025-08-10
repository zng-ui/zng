/*
https://learn.microsoft.com/en-us/typography/opentype/spec/otff
All OpenType fonts use Motorola-style byte ordering (Big Endian)

Fixed 	 = 32-bit signed fixed-point number (16.16)
Offset32 = uint32
NULL     = 0
 */

use core::cmp;

use byteorder::{BigEndian, ReadBytesExt};
use zng_view_api::font::GlyphIndex;

const GDEF: u32 = u32::from_be_bytes(*b"GDEF");

#[derive(Clone)]
pub struct LigatureCaretList {
    coverage: Coverage,
    lig_caret_start: Box<[u32]>,
    lig_carets: Box<[LigatureCaret]>,
}
impl LigatureCaretList {
    pub fn empty() -> Self {
        Self {
            coverage: Coverage::Format1 { glyphs: Box::new([]) },
            lig_caret_start: Box::new([]),
            lig_carets: Box::new([]),
        }
    }

    pub fn load(font: &ttf_parser::RawFace) -> std::io::Result<Self> {
        let table = match font.table(ttf_parser::Tag(GDEF)) {
            Some(d) => d,
            None => return Ok(Self::empty()),
        };

        /*
        https://learn.microsoft.com/en-us/typography/opentype/spec/gdef#gdef-header

        GDEF Header

        Type     Name                     Description
        uint16   majorVersion             Major version of the GDEF table, = 1
        uint16   minorVersion             Minor version of the GDEF table, = 0
        Offset16 glyphClassDefOffset      Offset to class definition table for glyph type, from beginning of GDEF header (may be NULL)
        Offset16 attachListOffset         Offset to attachment point list table, from beginning of GDEF header (may be NULL)
        Offset16 ligCaretListOffset       Offset to ligature caret list table, from beginning of GDEF header (may be NULL)
        ..
        */

        let mut cursor = std::io::Cursor::new(&table);

        let major_version = cursor.read_u16::<BigEndian>()?;
        let _minor_version = cursor.read_u16::<BigEndian>()?;
        if major_version != 1 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "unknown GDEF version"));
        }

        let _glyph_class_def_offset = cursor.read_u16::<BigEndian>()?;
        let _attach_list_offset = cursor.read_u16::<BigEndian>()?;

        let lig_caret_list_offset = cursor.read_u16::<BigEndian>()? as u64;

        if lig_caret_list_offset == 0 {
            return Ok(Self::empty());
        }

        cursor.set_position(lig_caret_list_offset);

        /*
        https://learn.microsoft.com/en-us/typography/opentype/spec/gdef#ligature-caret-list-table

        Ligature Caret List table (LigCaretList)

        Type     Name                           Description
        Offset16 coverageOffset                 Offset to Coverage table - from beginning of LigCaretList table
        uint16   ligGlyphCount                  Number of ligature glyphs
        Offset16 ligGlyphOffsets[ligGlyphCount] Array of offsets to LigGlyph tables, from beginning of LigCaretList table —in Coverage Index order
        */

        let coverage_offset = cursor.read_u16::<BigEndian>()? as u64;
        /*
        https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2#coverageTbl

        Coverage Table

        Coverage Format 1: Individual glyph indices
        Type   Name                   Description
        uint16 coverageFormat         Format identifier — format = 1
        uint16 glyphCount             Number of glyphs in the glyph array
        uint16 glyphArray[glyphCount] Array of glyph IDs — in numerical order

        Coverage Format 2: Range of glyphs
        Type        Name                     Description
        uint16      coverageFormat           Format identifier — format = 2
        uint16      rangeCount               Number of RangeRecords
        RangeRecord rangeRecords[rangeCount] Array of glyph ranges — ordered by startGlyphID.

        RangeRecord:
        Type   Name               Description
        uint16 startGlyphID       First glyph ID in the range
        uint16 endGlyphID         Last glyph ID in the range
        uint16 startCoverageIndex Coverage Index of first glyph ID in range
        */
        let return_offset = cursor.position();
        cursor.set_position(lig_caret_list_offset + coverage_offset);
        let coverage_format = cursor.read_u16::<BigEndian>()?;
        let coverage = match coverage_format {
            1 => {
                let glyph_count = cursor.read_u16::<BigEndian>()?;
                let mut glyphs = Vec::with_capacity(glyph_count as usize);
                for _ in 0..glyph_count {
                    glyphs.push(cursor.read_u16::<BigEndian>()?);
                }

                Coverage::Format1 {
                    glyphs: glyphs.into_boxed_slice(),
                }
            }
            2 => {
                let range_count = cursor.read_u16::<BigEndian>()?;
                let mut glyph_ranges = Vec::with_capacity(range_count as usize);
                for _ in 0..range_count {
                    glyph_ranges.push(RangeRecord {
                        start_glyph_id: cursor.read_u16::<BigEndian>()?,
                        end_glyph_id: cursor.read_u16::<BigEndian>()?,
                        start_coverage_index: cursor.read_u16::<BigEndian>()?,
                    });
                }
                Coverage::Format2 {
                    glyph_ranges: glyph_ranges.into_boxed_slice(),
                }
            }
            _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "unknown coverage format")),
        };
        cursor.set_position(return_offset);

        let lig_glyph_count = cursor.read_u16::<BigEndian>()?;
        let lig_glyph_offsets = cursor.read_u16::<BigEndian>()? as u64;

        cursor.set_position(lig_caret_list_offset + lig_glyph_offsets);

        let mut lig_caret_start = Vec::with_capacity(lig_glyph_count as usize);
        let mut lig_carets = vec![];

        for _ in 0..lig_glyph_count {
            /*
            https://learn.microsoft.com/en-us/typography/opentype/spec/gdef#ligature-glyph-table

            Ligature Glyph table (LigGlyph)

            Type     Name                          Description
            uint16   caretCount                    Number of CaretValue tables for this ligature (components - 1)
            Offset16 caretValueOffsets[caretCount] Array of offsets to CaretValue tables, from beginning of LigGlyph table — in increasing coordinate order
            */

            let caret_count = cursor.read_u16::<BigEndian>()?;
            let caret_value_offsets = cursor.read_u16::<BigEndian>()? as u64;

            if caret_count == 0 {
                continue;
            }
            lig_caret_start.push(lig_carets.len() as u32);
            lig_carets.reserve(caret_count as usize);

            let return_offset = cursor.position();
            cursor.set_position(caret_value_offsets);
            for _ in 0..caret_count {
                /*
                https://learn.microsoft.com/en-us/typography/opentype/spec/gdef#caret-value-tables

                Caret Values table (CaretValues)

                CaretValue Format 1
                Type   Name             Description
                uint16 CaretValueFormat Format identifier-format = 1
                int16  Coordinate       X or Y value, in design units

                CaretValue Format 2
                Type   Name                 Description
                uint16 CaretValueFormat     Format identifier-format = 2
                uint16 caretValuePointIndex Contour point index on glyph

                CaretValue Format 3
                Type     Name             Description
                uint16   CaretValueFormat Format identifier-format = 3
                int16    Coordinate       X or Y value, in design units
                Offset16 DeviceOffset     Offset to Device table for X or Y value-from beginning of CaretValue table
                */

                let caret_value_format = cursor.read_u16::<BigEndian>()?;

                match caret_value_format {
                    1 => {
                        let coordinate = cursor.read_i16::<BigEndian>()?;
                        lig_carets.push(LigatureCaret::Coordinate(coordinate));
                    }
                    2 => {
                        let caret_value_point_index = cursor.read_u16::<BigEndian>()?;
                        lig_carets.push(LigatureCaret::GlyphContourPoint(caret_value_point_index));
                    }
                    3 => {
                        let coordinate = cursor.read_i16::<BigEndian>()?;
                        lig_carets.push(LigatureCaret::Coordinate(coordinate));
                        let _device_table = cursor.read_u32::<BigEndian>()? as u64;
                    }
                    _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "unknown CaretValue format")),
                }
            }
            cursor.set_position(return_offset);
        }

        Ok(Self {
            coverage,
            lig_caret_start: lig_caret_start.into_boxed_slice(),
            lig_carets: lig_carets.into_boxed_slice(),
        })
    }

    /// Gets the caret offsets for clusters in the `lig` glyph, except the first cluster.
    ///
    /// The caret position for the first cluster is the ligature glyph position, the returned
    /// slice contains the carets for subsequent clusters that form the ligature.
    ///
    /// Returns an empty slice if the font does not provide caret positions for `lig`, in this
    /// case app must divide the glyph advance in equal parts to find caret positions.
    pub fn carets(&self, lig: GlyphIndex) -> &[LigatureCaret] {
        if let Some(p) = self.coverage.position(lig)
            && let Some(&start) = self.lig_caret_start.get(p)
        {
            let start = start as usize;
            let next_p = p + 1;
            return if next_p < self.lig_carets.len() {
                let end = self.lig_caret_start[next_p] as usize;
                &self.lig_carets[start..end]
            } else {
                &self.lig_carets[start..]
            };
        }
        &[]
    }

    /// If the font provides not ligature caret positions.
    ///
    /// If `true` the [`carets`] method always returns an empty slice.
    ///
    /// [`carets`]: Self::carets
    pub fn is_empty(&self) -> bool {
        match &self.coverage {
            Coverage::Format1 { glyphs } => glyphs.is_empty(),
            Coverage::Format2 { glyph_ranges } => glyph_ranges.is_empty(),
        }
    }
}

#[derive(Clone)]
enum Coverage {
    Format1 {
        /// Sorted glyph IDs, position is coverage index.
        glyphs: Box<[u16]>,
    },
    Format2 {
        /// Sorted glyph ranges.
        glyph_ranges: Box<[RangeRecord]>,
    },
}
impl Coverage {
    fn position(&self, glyph: GlyphIndex) -> Option<usize> {
        let glyph = glyph as u16;
        match self {
            Coverage::Format1 { glyphs } => glyphs.binary_search(&glyph).ok(),
            Coverage::Format2 { glyph_ranges } => {
                // see: https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2#coverage-format-2

                let i = glyph_ranges
                    .binary_search_by(|r| {
                        if glyph < r.start_glyph_id {
                            cmp::Ordering::Greater
                        } else if glyph <= r.end_glyph_id {
                            cmp::Ordering::Equal
                        } else {
                            cmp::Ordering::Less
                        }
                    })
                    .ok()?;
                let r = &glyph_ranges[i];

                Some((r.start_coverage_index + glyph - r.start_glyph_id) as usize)
            }
        }
    }
}

#[derive(Clone, Copy)]
struct RangeRecord {
    start_glyph_id: u16,
    /// Inclusive.
    end_glyph_id: u16,
    start_coverage_index: u16,
}

#[derive(Clone, Copy, Debug)]
pub enum LigatureCaret {
    /// Offset in font units.
    Coordinate(i16),
    /// Index of a point in the glyph outline.
    GlyphContourPoint(u16),
}
