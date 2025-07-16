//! This module contains copies of `unicode_bidi` code, with the dependency on text strings removed.
//!
//! Text is only used in the source crate to handle character with multiple bytes, but we do bidi sorting at
//! the "segment" level.

use std::{collections::HashMap, ops::Range};

use unicode_bidi::{BidiClass, BidiDataSource, Level};

pub(super) fn visual_runs(
    mut levels: Vec<unicode_bidi::Level>,
    line_classes: Vec<unicode_bidi::BidiClass>,
    para_level: unicode_bidi::Level,
) -> (Vec<unicode_bidi::Level>, Vec<unicode_bidi::LevelRun>) {
    use unicode_bidi::BidiClass::*;

    let line_levels = &mut levels;

    // Reset some whitespace chars to paragraph level.
    // <http://www.unicode.org/reports/tr9/#L1>
    let mut reset_from: Option<usize> = Some(0);
    let mut reset_to: Option<usize> = None;
    let mut prev_level = para_level;
    for i in 0..line_classes.len() {
        match line_classes[i] {
            // Segment separator, Paragraph separator
            B | S => {
                assert_eq!(reset_to, None);
                reset_to = Some(i + 1);
                if reset_from.is_none() {
                    reset_from = Some(i);
                }
            }
            // Whitespace, isolate formatting
            WS | FSI | LRI | RLI | PDI => {
                if reset_from.is_none() {
                    reset_from = Some(i);
                }
            }
            // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
            // same as above + set the level
            RLE | LRE | RLO | LRO | PDF | BN => {
                if reset_from.is_none() {
                    reset_from = Some(i);
                }
                // also set the level to previous
                line_levels[i] = prev_level;
            }
            _ => {
                reset_from = None;
            }
        }
        if let Some(from) = reset_from
            && let Some(to) = reset_to
        {
            for level in &mut line_levels[from..to] {
                *level = para_level;
            }
            reset_from = None;
            reset_to = None;
        }
        prev_level = line_levels[i];
    }
    if let Some(from) = reset_from {
        for level in &mut line_levels[from..] {
            *level = para_level;
        }
    }

    // Find consecutive level runs.
    let mut runs = Vec::new();
    let mut start = 0;
    let mut run_level = levels[start];
    let mut min_level = run_level;
    let mut max_level = run_level;

    for (i, &new_level) in levels.iter().enumerate().skip(1) {
        if new_level != run_level {
            // End of the previous run, start of a new one.
            runs.push(start..i);
            start = i;
            run_level = new_level;
            min_level = std::cmp::min(run_level, min_level);
            max_level = std::cmp::max(run_level, max_level);
        }
    }
    runs.push(start..line_classes.len());

    let run_count = runs.len();

    // Re-order the odd runs.
    // <http://www.unicode.org/reports/tr9/#L2>

    // Stop at the lowest *odd* level.
    min_level = min_level.new_lowest_ge_rtl().expect("Level error");

    while max_level >= min_level {
        // Look for the start of a sequence of consecutive runs of max_level or higher.
        let mut seq_start = 0;
        while seq_start < run_count {
            if levels[runs[seq_start].start] < max_level {
                seq_start += 1;
                continue;
            }

            // Found the start of a sequence. Now find the end.
            let mut seq_end = seq_start + 1;
            while seq_end < run_count {
                if levels[runs[seq_end].start] < max_level {
                    break;
                }
                seq_end += 1;
            }

            // Reverse the runs within this sequence.
            runs[seq_start..seq_end].reverse();

            seq_start = seq_end;
        }
        max_level.lower(1).expect("Lowering embedding level below zero");
    }

    (levels, runs)
}

pub(super) fn explicit_compute(
    para_level: Level,
    original_classes: &[BidiClass],
    levels: &mut [Level],
    processing_classes: &mut [BidiClass],
) {
    // <http://www.unicode.org/reports/tr9/#X1>
    let mut stack = DirectionalStatusStack::new();
    stack.push(para_level, OverrideStatus::Neutral);

    let mut overflow_isolate_count = 0u32;
    let mut overflow_embedding_count = 0u32;
    let mut valid_isolate_count = 0u32;

    for i in 0..original_classes.len() {
        use BidiClass::*;
        match original_classes[i] {
            // Rules X2-X5c
            RLE | LRE | RLO | LRO | RLI | LRI | FSI => {
                let last_level = stack.last().level;

                // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
                levels[i] = last_level;

                // X5a-X5c: Isolate initiators get the level of the last entry on the stack.
                let is_isolate = matches!(original_classes[i], RLI | LRI | FSI);
                if is_isolate {
                    // Redundant due to "Retaining explicit formatting characters" step.
                    // levels[i] = last_level;
                    match stack.last().status {
                        OverrideStatus::RTL => processing_classes[i] = R,
                        OverrideStatus::LTR => processing_classes[i] = L,
                        _ => {}
                    }
                }

                let new_level = if is_rtl(original_classes[i]) {
                    last_level.new_explicit_next_rtl()
                } else {
                    last_level.new_explicit_next_ltr()
                };

                if new_level.is_ok() && overflow_isolate_count == 0 && overflow_embedding_count == 0 {
                    let new_level = new_level.unwrap();
                    stack.push(
                        new_level,
                        match original_classes[i] {
                            RLO => OverrideStatus::RTL,
                            LRO => OverrideStatus::LTR,
                            RLI | LRI | FSI => OverrideStatus::Isolate,
                            _ => OverrideStatus::Neutral,
                        },
                    );
                    if is_isolate {
                        valid_isolate_count += 1;
                    } else {
                        // The spec doesn't explicitly mention this step, but it is necessary.
                        // See the reference implementations for comparison.
                        levels[i] = new_level;
                    }
                } else if is_isolate {
                    overflow_isolate_count += 1;
                } else if overflow_isolate_count == 0 {
                    overflow_embedding_count += 1;
                }

                if !is_isolate {
                    // X9 +
                    // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
                    // (PDF handled below)
                    processing_classes[i] = BN;
                }
            }

            // <http://www.unicode.org/reports/tr9/#X6a>
            PDI => {
                if overflow_isolate_count > 0 {
                    overflow_isolate_count -= 1;
                } else if valid_isolate_count > 0 {
                    overflow_embedding_count = 0;
                    loop {
                        // Pop everything up to and including the last Isolate status.
                        match stack.vec.pop() {
                            None
                            | Some(Status {
                                status: OverrideStatus::Isolate,
                                ..
                            }) => break,
                            _ => continue,
                        }
                    }
                    valid_isolate_count -= 1;
                }
                let last = stack.last();
                levels[i] = last.level;
                match last.status {
                    OverrideStatus::RTL => processing_classes[i] = R,
                    OverrideStatus::LTR => processing_classes[i] = L,
                    _ => {}
                }
            }

            // <http://www.unicode.org/reports/tr9/#X7>
            PDF => {
                if overflow_isolate_count > 0 {
                    // do nothing
                } else if overflow_embedding_count > 0 {
                    overflow_embedding_count -= 1;
                } else if stack.last().status != OverrideStatus::Isolate && stack.vec.len() >= 2 {
                    stack.vec.pop();
                }
                // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
                levels[i] = stack.last().level;
                // X9 part of retaining explicit formatting characters.
                processing_classes[i] = BN;
            }

            // Nothing.
            // BN case moved down to X6, see <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
            B => {}

            // <http://www.unicode.org/reports/tr9/#X6>
            _ => {
                let last = stack.last();
                levels[i] = last.level;
                // This condition is not in the spec, but I am pretty sure that is a spec bug.
                // https://www.unicode.org/L2/L2023/23014-amd-to-uax9.pdf
                if original_classes[i] != BN {
                    match last.status {
                        OverrideStatus::RTL => processing_classes[i] = R,
                        OverrideStatus::LTR => processing_classes[i] = L,
                        _ => {}
                    }
                }
            }
        }
    }
}

pub(super) fn prepare_isolating_run_sequences(
    para_level: Level,
    original_classes: &[BidiClass],
    levels: &[Level],
) -> Vec<IsolatingRunSequence> {
    let runs = level_runs(levels, original_classes);

    // Compute the set of isolating run sequences.
    // <http://www.unicode.org/reports/tr9/#BD13>
    let mut sequences = Vec::with_capacity(runs.len());

    // When we encounter an isolate initiator, we push the current sequence onto the
    // stack so we can resume it after the matching PDI.
    let mut stack = vec![Vec::new()];

    use BidiClass::*;

    for run in runs {
        assert!(!run.is_empty());
        assert!(!stack.is_empty());

        let start_class = original_classes[run.start];
        let end_class = original_classes[run.end - 1];

        let mut sequence = if start_class == PDI && stack.len() > 1 {
            // Continue a previous sequence interrupted by an isolate.
            stack.pop().unwrap()
        } else {
            // Start a new sequence.
            Vec::new()
        };

        sequence.push(run);

        if let RLI | LRI | FSI = end_class {
            // Resume this sequence after the isolate.
            stack.push(sequence);
        } else {
            // This sequence is finished.
            sequences.push(sequence);
        }
    }
    // Pop any remaining sequences off the stack.
    sequences.extend(stack.into_iter().rev().filter(|seq| !seq.is_empty()));

    // Determine the `sos` and `eos` class for each sequence.
    // <http://www.unicode.org/reports/tr9/#X10>
    sequences
        .into_iter()
        .map(|sequence: Vec<LevelRun>| {
            assert!(!sequence.is_empty());

            let mut result = IsolatingRunSequence {
                runs: sequence,
                sos: L,
                eos: L,
            };

            let start_of_seq = result.runs[0].start;
            let runs_len = result.runs.len();
            let end_of_seq = result.runs[runs_len - 1].end;

            // > (not counting characters removed by X9)
            let seq_level = result
                .iter_forwards_from(start_of_seq, 0)
                .filter(|i| not_removed_by_x9(&original_classes[*i]))
                .map(|i| levels[i])
                .next()
                .unwrap_or(levels[start_of_seq]);

            let end_level = result
                .iter_backwards_from(end_of_seq, runs_len - 1)
                .filter(|i| not_removed_by_x9(&original_classes[*i]))
                .map(|i| levels[i])
                .next()
                .unwrap_or(levels[end_of_seq - 1]);

            #[cfg(test)]
            for run in result.runs.clone() {
                for idx in run {
                    if not_removed_by_x9(&original_classes[idx]) {
                        assert_eq!(seq_level, levels[idx]);
                    }
                }
            }

            // Get the level of the last non-removed char before the runs.
            let pred_level = match original_classes[..start_of_seq].iter().rposition(not_removed_by_x9) {
                Some(idx) => levels[idx],
                None => para_level,
            };

            // Get the last non-removed character to check if it is an isolate initiator.
            // The spec calls for an unmatched one, but matched isolate initiators
            // will never be at the end of a level run (otherwise there would be more to the run).
            // We unwrap_or(BN) because BN marks removed classes and it won't matter for the check.
            let last_non_removed = original_classes[..end_of_seq]
                .iter()
                .copied()
                .rev()
                .find(not_removed_by_x9)
                .unwrap_or(BN);

            // Get the level of the next non-removed char after the runs.
            let succ_level = if let RLI | LRI | FSI = last_non_removed {
                para_level
            } else {
                match original_classes[end_of_seq..].iter().position(not_removed_by_x9) {
                    Some(idx) => levels[end_of_seq + idx],
                    None => para_level,
                }
            };

            result.sos = std::cmp::max(seq_level, pred_level).bidi_class();
            result.eos = std::cmp::max(end_level, succ_level).bidi_class();
            result
        })
        .collect()
}

/// Entries in the directional status stack:
struct Status {
    level: Level,
    status: OverrideStatus,
}

#[derive(PartialEq)]
#[expect(clippy::upper_case_acronyms)]
enum OverrideStatus {
    Neutral,
    RTL,
    LTR,
    Isolate,
}

struct DirectionalStatusStack {
    vec: Vec<Status>,
}

impl DirectionalStatusStack {
    fn new() -> Self {
        DirectionalStatusStack {
            vec: Vec::with_capacity(Level::max_explicit_depth() as usize + 2),
        }
    }

    fn push(&mut self, level: Level, status: OverrideStatus) {
        self.vec.push(Status { level, status });
    }

    fn last(&self) -> &Status {
        self.vec.last().unwrap()
    }
}

fn is_rtl(bidi_class: BidiClass) -> bool {
    use BidiClass::*;
    matches!(bidi_class, RLE | RLO | RLI)
}

pub(super) type LevelRun = Range<usize>;

pub(super) struct IsolatingRunSequence {
    pub runs: Vec<LevelRun>,
    pub sos: BidiClass, // Start-of-sequence type.
    pub eos: BidiClass, // End-of-sequence type.
}

/// Should this character be ignored in steps after X9?
///
/// <http://www.unicode.org/reports/tr9/#X9>
fn removed_by_x9(class: BidiClass) -> bool {
    use BidiClass::*;
    matches!(class, RLE | LRE | RLO | LRO | PDF | BN)
}

// For use as a predicate for `position` / `r_position`
fn not_removed_by_x9(class: &BidiClass) -> bool {
    !removed_by_x9(*class)
}

impl IsolatingRunSequence {
    /// Given a text-relative position `pos` and an index of the level run it is in,
    /// produce an iterator of all characters after and pos (`pos..`) that are in this
    /// run sequence
    pub(crate) fn iter_forwards_from(&self, pos: usize, level_run_index: usize) -> impl Iterator<Item = usize> + '_ {
        let runs = &self.runs[level_run_index..];

        // Check that it is in range
        // (we can't use contains() since we want an inclusive range)
        debug_assert!(runs[0].start <= pos && pos <= runs[0].end);

        (pos..runs[0].end).chain(runs[1..].iter().flat_map(Clone::clone))
    }

    /// Given a text-relative position `pos` and an index of the level run it is in,
    /// produce an iterator of all characters before and excluding pos (`..pos`) that are in this
    /// run sequence
    pub(crate) fn iter_backwards_from(&self, pos: usize, level_run_index: usize) -> impl Iterator<Item = usize> + '_ {
        let prev_runs = &self.runs[..level_run_index];
        let current = &self.runs[level_run_index];

        // Check that it is in range
        // (we can't use contains() since we want an inclusive range)
        debug_assert!(current.start <= pos && pos <= current.end);

        (current.start..pos).rev().chain(prev_runs.iter().rev().flat_map(Clone::clone))
    }
}

/// Finds the level runs in a paragraph.
///
/// <http://www.unicode.org/reports/tr9/#BD7>
fn level_runs(levels: &[Level], original_classes: &[BidiClass]) -> Vec<LevelRun> {
    assert_eq!(levels.len(), original_classes.len());

    let mut runs = Vec::new();
    if levels.is_empty() {
        return runs;
    }

    let mut current_run_level = levels[0];
    let mut current_run_start = 0;
    for i in 1..levels.len() {
        if !removed_by_x9(original_classes[i]) && levels[i] != current_run_level {
            // End the last run and start a new one.
            runs.push(current_run_start..i);
            current_run_level = levels[i];
            current_run_start = i;
        }
    }
    runs.push(current_run_start..levels.len());

    runs
}

/// 3.3.4 Resolving Weak Types
///
/// <http://www.unicode.org/reports/tr9/#Resolving_Weak_Types>
pub(super) fn implicit_resolve_weak(sequence: &IsolatingRunSequence, processing_classes: &mut [BidiClass]) {
    use BidiClass::*;

    // Note: The spec treats these steps as individual passes that are applied one after the other
    // on the entire IsolatingRunSequence at once. We instead collapse it into a single iteration,
    // which is straightforward for rules that are based on the state of the current character, but not
    // for rules that care about surrounding characters. To deal with them, we retain additional state
    // about previous character classes that may have since been changed by later rules.

    // The previous class for the purposes of rule W4/W6, not tracking changes made after or during W4.
    let mut prev_class_before_w4 = sequence.sos;
    // The previous class for the purposes of rule W5.
    let mut prev_class_before_w5 = sequence.sos;
    // The previous class for the purposes of rule W1, not tracking changes from any other rules.
    let mut prev_class_before_w1 = sequence.sos;
    let mut last_strong_is_al = false;
    let mut et_run_indices = Vec::new(); // for W5
    let mut bn_run_indices = Vec::new(); // for W5 + <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>

    for (run_index, level_run) in sequence.runs.iter().enumerate() {
        for i in &mut level_run.clone() {
            if processing_classes[i] == BN {
                // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
                // Keeps track of BN runs for W5 in case we see an ET.
                bn_run_indices.push(i);
                // BNs aren't real, skip over them.
                continue;
            }

            // Store the processing class of all rules before W2/W1.
            // Used to keep track of the last strong character for W2. W3 is able to insert new strong
            // characters, so we don't want to be misled by it.
            let mut w2_processing_class = processing_classes[i];

            // <http://www.unicode.org/reports/tr9/#W1>
            //

            if processing_classes[i] == NSM {
                processing_classes[i] = match prev_class_before_w1 {
                    RLI | LRI | FSI | PDI => ON,
                    _ => prev_class_before_w1,
                };
                // W1 occurs before W2, update this.
                w2_processing_class = processing_classes[i];
            }

            prev_class_before_w1 = processing_classes[i];

            // <http://www.unicode.org/reports/tr9/#W2>
            // <http://www.unicode.org/reports/tr9/#W3>
            //
            match processing_classes[i] {
                EN => {
                    if last_strong_is_al {
                        // W2. If previous strong char was AL, change EN to AN.
                        processing_classes[i] = AN;
                    }
                }
                // W3.
                AL => processing_classes[i] = R,
                _ => {}
            }

            // update last_strong_is_al.
            match w2_processing_class {
                L | R => {
                    last_strong_is_al = false;
                }
                AL => {
                    last_strong_is_al = true;
                }
                _ => {}
            }

            let class_before_w456 = processing_classes[i];

            // <http://www.unicode.org/reports/tr9/#W4>
            // <http://www.unicode.org/reports/tr9/#W5>
            // <http://www.unicode.org/reports/tr9/#W6> (separators only)
            // (see below for W6 terminator code)
            //
            match processing_classes[i] {
                // <http://www.unicode.org/reports/tr9/#W6>
                EN => {
                    // W5. If a run of ETs is adjacent to an EN, change the ETs to EN.
                    for j in &et_run_indices {
                        processing_classes[*j] = EN;
                    }
                    et_run_indices.clear();
                }

                // <http://www.unicode.org/reports/tr9/#W4>
                // <http://www.unicode.org/reports/tr9/#W6>
                ES | CS => {
                    let mut next_class = sequence
                        .iter_forwards_from(i, run_index)
                        .map(|j| processing_classes[j])
                        // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
                        .find(not_removed_by_x9)
                        .unwrap_or(sequence.eos);
                    if next_class == EN && last_strong_is_al {
                        // Apply W2 to next_class. We know that last_strong_is_al
                        // has no chance of changing on this character so we can still assume its value
                        // will be the same by the time we get to it.
                        next_class = AN;
                    }
                    processing_classes[i] = match (prev_class_before_w4, processing_classes[i], next_class) {
                        // W4
                        (EN, ES, EN) | (EN, CS, EN) => EN,
                        // W4
                        (AN, CS, AN) => AN,
                        // W6 (separators only)
                        (_, _, _) => ON,
                    };

                    // W6 + <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
                    // We have to do this before W5 gets its grubby hands on these characters and thinks
                    // they're part of an ET run.
                    // We check for ON to ensure that we had hit the W6 branch above, since this `ES | CS` match
                    // arm handles both W4 and W6.
                    if processing_classes[i] == ON {
                        for idx in sequence.iter_backwards_from(i, run_index) {
                            let class = &mut processing_classes[idx];
                            if *class != BN {
                                break;
                            }
                            *class = ON;
                        }
                        for idx in sequence.iter_forwards_from(i, run_index) {
                            let class = &mut processing_classes[idx];
                            if *class != BN {
                                break;
                            }
                            *class = ON;
                        }
                    }
                }
                // <http://www.unicode.org/reports/tr9/#W5>
                ET => {
                    match prev_class_before_w5 {
                        EN => processing_classes[i] = EN,
                        _ => {
                            // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
                            // If there was a BN run before this, that's now a part of this ET run.
                            et_run_indices.extend(&bn_run_indices);

                            // In case this is followed by an EN.
                            et_run_indices.push(i);
                        }
                    }
                }
                _ => {}
            }

            // Common loop iteration code
            //

            // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
            // BN runs would have already continued the loop, clear them before we get to the next one.
            bn_run_indices.clear();

            // W6 above only deals with separators, so it doesn't change anything W5 cares about,
            // so we still can update this after running that part of W6.
            prev_class_before_w5 = processing_classes[i];

            // <http://www.unicode.org/reports/tr9/#W6> (terminators only)
            // (see above for W6 separator code)
            //
            if prev_class_before_w5 != ET {
                // W6. If we didn't find an adjacent EN, turn any ETs into ON instead.
                for j in &et_run_indices {
                    processing_classes[*j] = ON;
                }
                et_run_indices.clear();
            }

            // We stashed this before W4/5/6 could get their grubby hands on it, and it's not
            // used in the W6 terminator code below so we can update it now.
            prev_class_before_w4 = class_before_w456;
        }
    }
    // Rerun this check in case we ended with a sequence of BNs (i.e., we'd never
    // hit the end of the for loop above).
    // W6. If we didn't find an adjacent EN, turn any ETs into ON instead.
    for j in &et_run_indices {
        processing_classes[*j] = ON;
    }
    et_run_indices.clear();

    // W7. If the previous strong char was L, change EN to L.
    let mut last_strong_is_l = sequence.sos == L;
    for run in &sequence.runs {
        for i in run.clone() {
            match processing_classes[i] {
                EN if last_strong_is_l => {
                    processing_classes[i] = L;
                }
                L => {
                    last_strong_is_l = true;
                }
                R | AL => {
                    last_strong_is_l = false;
                }
                // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
                // Already scanning past BN here.
                _ => {}
            }
        }
    }
}

/// 3.3.5 Resolving Neutral Types
///
/// <http://www.unicode.org/reports/tr9/#Resolving_Neutral_Types>
pub(super) fn implicit_resolve_neutral(
    sequence: &IsolatingRunSequence,
    levels: &[Level],
    original_classes: &[BidiClass],
    processing_classes: &mut [BidiClass],
    brackets: &HashMap<usize, char>,
) {
    use BidiClass::*;

    // e = embedding direction
    let e: BidiClass = levels[sequence.runs[0].start].bidi_class();
    let not_e = if e == BidiClass::L { BidiClass::R } else { BidiClass::L };
    // N0. Process bracket pairs.

    // > Identify the bracket pairs in the current isolating run sequence according to BD16.
    // We use processing_classes, not original_classes, due to BD14/BD15
    let bracket_pairs = identify_bracket_pairs(sequence, processing_classes, brackets);

    // > For each bracket-pair element in the list of pairs of text positions
    //
    // Note: Rust ranges are interpreted as [start..end), be careful using `pair` directly
    // for indexing as it will include the opening bracket pair but not the closing one.
    for pair in bracket_pairs {
        debug_assert!(
            pair.start < processing_classes.len(),
            "identify_bracket_pairs returned a range that is out of bounds!"
        );
        debug_assert!(
            pair.end < processing_classes.len(),
            "identify_bracket_pairs returned a range that is out of bounds!"
        );
        let mut found_e = false;
        let mut found_not_e = false;
        let mut class_to_set = None;

        // > Inspect the bidirectional types of the characters enclosed within the bracket pair.
        //
        // `pair` is [start, end) so we will end up processing the opening character but not the closing one.
        //
        for enclosed_i in sequence.iter_forwards_from(pair.start + 1, pair.start_run) {
            if enclosed_i >= pair.end {
                debug_assert!(enclosed_i == pair.end, "If we skipped past this, the iterator is broken");
                break;
            }
            let class = processing_classes[enclosed_i];
            if class == e {
                found_e = true;
            } else if class == not_e {
                found_not_e = true;
            } else if class == BidiClass::EN || class == BidiClass::AN {
                // > Within this scope, bidirectional types EN and AN are treated as R.
                if e == BidiClass::L {
                    found_not_e = true;
                } else {
                    found_e = true;
                }
            }

            // If we have found a character with the class of the embedding direction
            // we can bail early.
            if found_e {
                break;
            }
        }
        // > If any strong type (either L or R) matching the embedding direction is found
        if found_e {
            // > .. set the type for both brackets in the pair to match the embedding direction
            class_to_set = Some(e);
        // > Otherwise, if there is a strong type it must be opposite the embedding direction
        } else if found_not_e {
            // > Therefore, test for an established context with a preceding strong type by
            // > checking backwards before the opening paired bracket
            // > until the first strong type (L, R, or sos) is found.
            // (see note above about processing_classes and character boundaries)
            let mut previous_strong = sequence
                .iter_backwards_from(pair.start, pair.start_run)
                .map(|i| processing_classes[i])
                .find(|class| *class == BidiClass::L || *class == BidiClass::R || *class == BidiClass::EN || *class == BidiClass::AN)
                .unwrap_or(sequence.sos);

            // > Within this scope, bidirectional types EN and AN are treated as R.
            if previous_strong == BidiClass::EN || previous_strong == BidiClass::AN {
                previous_strong = BidiClass::R;
            }

            // > If the preceding strong type is also opposite the embedding direction,
            // > context is established,
            // > so set the type for both brackets in the pair to that direction.
            // AND
            // > Otherwise set the type for both brackets in the pair to the embedding direction.
            // > Either way it gets set to previous_strong
            //
            // Both branches amount to setting the type to the strong type.
            class_to_set = Some(previous_strong);
        }

        if let Some(class_to_set) = class_to_set {
            // Update all processing classes corresponding to the start and end elements, as requested.
            // We should include all bytes of the character, not the first one.
            for class in &mut processing_classes[pair.start..pair.start + 1] {
                *class = class_to_set;
            }
            for class in &mut processing_classes[pair.end..pair.end + 1] {
                *class = class_to_set;
            }
            // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
            for idx in sequence.iter_backwards_from(pair.start, pair.start_run) {
                let class = &mut processing_classes[idx];
                if *class != BN {
                    break;
                }
                *class = class_to_set;
            }
            // > Any number of characters that had original bidirectional character type NSM prior to the application of
            // > W1 that immediately follow a paired bracket which changed to L or R under N0 should change to match the type of their preceding bracket.

            // This rule deals with sequences of NSMs, so we can just update them all at once, we don't need to worry
            // about character boundaries. We do need to be careful to skip the full set of bytes for the parentheses characters.
            let nsm_start = pair.start + 1;
            for idx in sequence.iter_forwards_from(nsm_start, pair.start_run) {
                let class = original_classes[idx];
                if class == BidiClass::NSM || processing_classes[idx] == BN {
                    processing_classes[idx] = class_to_set;
                } else {
                    break;
                }
            }
            let nsm_end = pair.end + 1;
            for idx in sequence.iter_forwards_from(nsm_end, pair.end_run) {
                let class = original_classes[idx];
                if class == BidiClass::NSM || processing_classes[idx] == BN {
                    processing_classes[idx] = class_to_set;
                } else {
                    break;
                }
            }
        }
        // > Otherwise, there are no strong types within the bracket pair
        // > Therefore, do not set the type for that bracket pair
    }

    // N1 and N2.
    // Indices of every byte in this isolating run sequence
    let mut indices = sequence.runs.iter().flat_map(Clone::clone);
    let mut prev_class = sequence.sos;
    while let Some(mut i) = indices.next() {
        // Process sequences of NI characters.
        let mut ni_run = Vec::new();
        // The BN is for <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
        if is_NI(processing_classes[i]) || processing_classes[i] == BN {
            // Consume a run of consecutive NI characters.
            ni_run.push(i);
            let mut next_class;
            loop {
                match indices.next() {
                    Some(j) => {
                        i = j;
                        next_class = processing_classes[j];
                        // The BN is for <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters>
                        if is_NI(next_class) || next_class == BN {
                            ni_run.push(i);
                        } else {
                            break;
                        }
                    }
                    None => {
                        next_class = sequence.eos;
                        break;
                    }
                };
            }
            // N1-N2.
            //
            // <http://www.unicode.org/reports/tr9/#N1>
            // <http://www.unicode.org/reports/tr9/#N2>
            let new_class = match (prev_class, next_class) {
                (L, L) => L,
                (R, R) | (R, AN) | (R, EN) | (AN, R) | (AN, AN) | (AN, EN) | (EN, R) | (EN, AN) | (EN, EN) => R,
                (_, _) => e,
            };
            for j in &ni_run {
                processing_classes[*j] = new_class;
            }
            ni_run.clear();
        }
        prev_class = processing_classes[i];
    }
}

struct BracketPair {
    /// The text-relative index of the opening bracket.
    start: usize,
    /// The text-relative index of the closing bracket.
    end: usize,
    /// The index of the run (in the run sequence) that the opening bracket is in.
    start_run: usize,
    /// The index of the run (in the run sequence) that the closing bracket is in.
    end_run: usize,
}
/// 3.1.3 Identifying Bracket Pairs
///
/// Returns all paired brackets in the source, as indices into the
/// text source.
///
/// <https://www.unicode.org/reports/tr9/#BD16>
fn identify_bracket_pairs(
    run_sequence: &IsolatingRunSequence,
    original_classes: &[BidiClass],
    brackets: &HashMap<usize, char>,
) -> Vec<BracketPair> {
    let data_source = &unicode_bidi::HardcodedBidiData;

    let mut ret = vec![];
    let mut stack = vec![];

    for (run_index, level_run) in run_sequence.runs.iter().enumerate() {
        for i in 0..level_run.len() {
            let actual_index = level_run.start + i;
            // All bracket characters are ON.
            // From BidiBrackets.txt:
            // > The Unicode property value stability policy guarantees that characters
            // > which have bpt=o or bpt=c also have bc=ON and Bidi_M=Y
            if original_classes[actual_index] != BidiClass::ON {
                continue;
            }

            if let Some(matched) = brackets
                .get(&actual_index)
                .and_then(|c| data_source.bidi_matched_opening_bracket(*c))
            {
                if matched.is_open {
                    // > If an opening paired bracket is found ...

                    // > ... and there is no room in the stack,
                    // > stop processing BD16 for the remainder of the isolating run sequence.
                    if stack.len() >= 63 {
                        break;
                    }
                    // > ... push its Bidi_Paired_Bracket property value and its text position onto the stack
                    stack.push((matched.opening, actual_index, run_index))
                } else {
                    // > If a closing paired bracket is found, do the following

                    // > Declare a variable that holds a reference to the current stack element
                    // > and initialize it with the top element of the stack.
                    // AND
                    // > Else, if the current stack element is not at the bottom of the stack
                    for (stack_index, element) in stack.iter().enumerate().rev() {
                        // > Compare the closing paired bracket being inspected or its canonical
                        // > equivalent to the bracket in the current stack element.
                        if element.0 == matched.opening {
                            // > If the values match, meaning the two characters form a bracket pair, then

                            // > Append the text position in the current stack element together with the
                            // > text position of the closing paired bracket to the list.
                            let pair = BracketPair {
                                start: element.1,
                                end: actual_index,
                                start_run: element.2,
                                end_run: run_index,
                            };
                            ret.push(pair);

                            // > Pop the stack through the current stack element inclusively.
                            stack.truncate(stack_index);
                            break;
                        }
                    }
                }
            }
        }
    }
    // > Sort the list of pairs of text positions in ascending order based on
    // > the text position of the opening paired bracket.
    ret.sort_by_key(|r| r.start);
    ret
}

/// Neutral or Isolate formatting character (B, S, WS, ON, FSI, LRI, RLI, PDI)
///
/// <http://www.unicode.org/reports/tr9/#NI>
#[expect(non_snake_case)]
fn is_NI(class: BidiClass) -> bool {
    use BidiClass::*;
    matches!(class, B | S | WS | ON | FSI | LRI | RLI | PDI)
}

/// 3.3.6 Resolving Implicit Levels
///
/// Returns the maximum embedding level in the paragraph.
///
/// <http://www.unicode.org/reports/tr9/#Resolving_Implicit_Levels>
pub(super) fn implicit_resolve_levels(original_classes: &[BidiClass], levels: &mut [Level]) -> Level {
    use BidiClass::*;

    let mut max_level = Level::ltr();
    assert_eq!(original_classes.len(), levels.len());
    for i in 0..levels.len() {
        match (levels[i].is_rtl(), original_classes[i]) {
            (false, AN) | (false, EN) => levels[i].raise(2).expect("Level number error"),
            (false, R) | (true, L) | (true, EN) | (true, AN) => levels[i].raise(1).expect("Level number error"),
            // <https://www.unicode.org/reports/tr9/#Retaining_Explicit_Formatting_Characters> handled here
            (_, _) => {}
        }
        max_level = std::cmp::max(max_level, levels[i]);
    }

    max_level
}

/// Assign levels to characters removed by rule X9.
///
/// The levels assigned to these characters are not specified by the algorithm. This function
/// assigns each one the level of the previous character, to avoid breaking level runs.
pub(super) fn assign_levels_to_removed_chars(para_level: Level, classes: &[BidiClass], levels: &mut [Level]) {
    for i in 0..levels.len() {
        if removed_by_x9(classes[i]) {
            levels[i] = if i > 0 { levels[i - 1] } else { para_level };
        }
    }
}
