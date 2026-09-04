use std::fmt;

// The frame, from the Unicode box-drawing block. Every piece is named rather than written at the
// point of use: `┬` and `┴` differ by a stroke, and a table drawn with the wrong one is a defect
// that reads as correct.
const HORIZONTAL: char = '─';
const VERTICAL: char = '│';
const TOP_LEFT: char = '┌';
const TOP_RIGHT: char = '┐';
const BOTTOM_LEFT: char = '└';
const BOTTOM_RIGHT: char = '┘';
const LEFT_TEE: char = '├';
const RIGHT_TEE: char = '┤';
const TOP_TEE: char = '┬';
const BOTTOM_TEE: char = '┴';

/// A framed table of two columns, under a title spanning the whole of it.
///
/// ```text
/// ┌──────────┬──────────┐
/// │ title    │          │   ← the title spans both columns and the divider between them
/// ├──────────┬──────────┤
/// │ label    │ value    │
/// └──────────┴──────────┘
/// ```
///
/// Each column takes the width of its widest cell, so nothing is fixed once and left to rot, and
/// a title wider than the two columns together stretches the value column rather than overflowing
/// the frame.
///
/// The table formats and returns lines; it never writes them. Who writes them, and whether they
/// are written once or rewritten in place, belongs to the caller — that is what lets the same
/// table serve a configuration printed once and a progress report redrawn every second.
pub struct Table {
    title: String,
    rows: Vec<(String, String)>,
}

impl Table {
    pub fn new(title: &str) -> Self {
        Table {
            title: title.to_string(),
            rows: Vec::new(),
        }
    }

    /// Adds a row, its label in the left column and its value in the right.
    ///
    /// A value holding newlines makes a row as tall as it is, the label on the row's middle line
    /// and nothing on the others — which is what lets a cell carry something framed on its own.
    pub fn push(&mut self, label: &str, value: &str) {
        self.rows.push((label.to_string(), value.to_string()));
    }

    /// The drawn lines, frame included, in order and without their line terminators.
    pub fn lines(&self) -> Vec<String> {
        let rows: Vec<(&str, Vec<&str>)> = self.rows.iter().map(|(label, value)| (label.as_str(), segments(value))).collect();

        let label_column = rows.iter().map(|(label, _)| display_width(label)).max().unwrap_or(0);
        let widest_value = rows
            .iter()
            .flat_map(|(_, segments)| segments.iter())
            .map(|segment| display_width(segment))
            .max()
            .unwrap_or(0);
        // The title spans both columns and the divider between them: two cells with a space on
        // each side, and one vertical between them, is `label + value + 3`.
        let value_column = widest_value.max(display_width(&self.title).saturating_sub(label_column + 3));
        let title_span = label_column + value_column + 3;

        let label_rule: String = std::iter::repeat(HORIZONTAL).take(label_column + 2).collect();
        let value_rule: String = std::iter::repeat(HORIZONTAL).take(value_column + 2).collect();

        let mut lines = vec![
            format!(
                "{TOP_LEFT}{}{TOP_RIGHT}",
                std::iter::repeat(HORIZONTAL).take(title_span + 2).collect::<String>()
            ),
            format!("{VERTICAL} {} {VERTICAL}", pad(&self.title, title_span)),
            format!("{LEFT_TEE}{label_rule}{TOP_TEE}{value_rule}{RIGHT_TEE}"),
        ];
        for (label, segments) in &rows {
            // The label sits on the row's middle line, not its first: a tall cell — a boxed gauge
            // is three lines — should carry its name beside its content rather than above it. An
            // even count has no middle, and the label leans to the line above, as a caption does.
            let label_line = (segments.len() - 1) / 2;

            for (index, segment) in segments.iter().enumerate() {
                let label = if index == label_line { *label } else { "" };
                lines.push(format!(
                    "{VERTICAL} {} {VERTICAL} {} {VERTICAL}",
                    pad(label, label_column),
                    pad(segment, value_column)
                ));
            }
        }
        lines.push(format!("{BOTTOM_LEFT}{label_rule}{BOTTOM_TEE}{value_rule}{BOTTOM_RIGHT}"));

        lines
    }
}

/// The three lines of a frame drawn tight around `content`, which must hold no newline.
///
/// This is how a cell gets a border of its own inside a table. A gauge needs one: its frame has
/// to mark where the track ends, and the cell's own border ends further right, past the
/// percentage. Nothing is inserted between the frame and the content — a fill that stopped short
/// of the border could not read as complete.
///
/// The pieces come from here rather than from the caller so that one module keeps the whole
/// box-drawing vocabulary.
pub fn boxed(content: &str) -> Vec<String> {
    let rule: String = std::iter::repeat(HORIZONTAL).take(display_width(content)).collect();

    vec![
        format!("{TOP_LEFT}{rule}{TOP_RIGHT}"),
        format!("{VERTICAL}{content}{VERTICAL}"),
        format!("{BOTTOM_LEFT}{rule}{BOTTOM_RIGHT}"),
    ]
}

/// The lines of a cell, an empty cell counting as one line rather than none.
fn segments(value: &str) -> Vec<&str> {
    let lines: Vec<&str> = value.lines().collect();

    if lines.is_empty() {
        vec![""]
    }
    else {
        lines
    }
}

/// The lines, joined by newlines and with none at the end — a block to hand to `println!`.
impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.lines().join("\n"))
    }
}

/// `cell`, followed by the spaces that bring it to `width` columns.
///
/// The `{:<width$}` of `format!` cannot do this: it counts characters, and a cell carrying colour
/// escapes has more of them than it occupies.
fn pad(cell: &str, width: usize) -> String {
    let padding = width.saturating_sub(display_width(cell));
    format!("{cell}{}", " ".repeat(padding))
}

/// The columns a cell takes once drawn, which is neither its byte length nor its character count.
///
/// A cell may carry SGR colour sequences — `ESC [ … m`, ECMA-48 §8.3.117 — and those move no
/// cursor. Counting their characters would pad a coloured cell short by as many columns, and the
/// frame would lean by exactly that much. Only SGR is skipped: it is the only kind of escape a
/// cell has any business holding, and a cursor movement inside one would break far more than an
/// alignment.
///
/// Bytes are not columns either, since the frame and the gauge are three bytes per character.
///
/// Public because composing a cell out of several lines means lining them up, and lining them up
/// means measuring them the way the table will.
pub fn display_width(cell: &str) -> usize {
    let mut width = 0;
    let mut characters = cell.chars();

    while let Some(character) = characters.next() {
        if character != '\x1b' {
            width += 1;
            continue;
        }
        // Swallow the sequence up to and including its final byte.
        for escaped in characters.by_ref() {
            if escaped == 'm' {
                break;
            }
        }
    }

    width
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREEN: &str = "\x1b[32m";
    const RESET: &str = "\x1b[0m";

    fn two_rows() -> Table {
        let mut table = Table::new("A title");
        table.push("label", "value");
        table.push("a longer label", "v");
        table
    }

    /// Every line of a table is the same width, which is what makes it look like a table at all.
    #[test]
    fn every_line_is_as_wide_as_every_other() {
        let lines = two_rows().lines();
        let width = display_width(&lines[0]);

        for line in &lines {
            assert_eq!(width, display_width(line), "{line}");
        }
    }

    /// The reason [`display_width`] exists: a coloured cell holds characters that take no column,
    /// and padding it by its character count would pull the right-hand border in.
    #[test]
    fn a_coloured_cell_does_not_lean_the_frame() {
        let mut coloured = Table::new("A title");
        coloured.push("label", &format!("{GREEN}value{RESET}"));

        let mut plain = Table::new("A title");
        plain.push("label", "value");

        assert_eq!(plain.lines().len(), coloured.lines().len());
        for (plain, coloured) in plain.lines().iter().zip(coloured.lines().iter()) {
            assert_eq!(display_width(plain), display_width(coloured));
        }
    }

    #[test]
    fn a_title_wider_than_the_rows_stretches_the_table_rather_than_overflowing_it() {
        let mut table = Table::new("A title far wider than any row of this table");
        table.push("a", "b");
        let lines = table.lines();

        assert_eq!(display_width(&lines[0]), display_width(&lines[1]));
        assert!(lines[1].contains("A title far wider than any row of this table"));
    }

    /// A row is the only line carrying two cells, so the frame and the title tell themselves
    /// apart from it — which is how the configuration round-trip test reads a table back.
    #[test]
    fn a_row_carries_two_cells_and_the_frame_carries_none() {
        let lines = two_rows().lines();

        assert_eq!(4, lines[3].split(VERTICAL).count());
        assert_eq!(3, lines[1].split(VERTICAL).count());
        assert_eq!(1, lines[0].split(VERTICAL).count());
    }

    /// A tall cell carries its label beside its content, not above it — which for three lines
    /// means the middle one.
    #[test]
    fn a_tall_row_labels_its_middle_line() {
        let mut table = Table::new("A title");
        table.push("tall", "one\ntwo\nthree");
        let lines = table.lines();

        assert!(!lines[3].contains("tall"));
        assert!(lines[4].contains("tall"));
        assert!(!lines[5].contains("tall"));
    }

    #[test]
    fn display_width_counts_columns_and_neither_bytes_nor_characters() {
        assert_eq!(5, display_width("value"));
        assert_eq!(5, display_width(&format!("{GREEN}value{RESET}")));
        // Three bytes, one column.
        assert_eq!(1, display_width("│"));
    }
}
