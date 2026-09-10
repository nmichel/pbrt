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

/// The divider between the two columns is one column wide, as every piece of the frame is.
const DIVIDER_WIDTH: usize = 1;

/// What the tables of this module are drawn with.
const PADDING: Padding = Padding { horizontal: 1, vertical: 0 };

/// A framed table of two columns, under a title spanning the whole of it.
///
/// ```text
/// ┌─────────────────────┐
/// │ title               │   ← no divider here: the title spans both columns and the one below
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
    rows: Vec<Row>,
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
    /// Either cell may hold newlines, and the row is then as tall as the taller of the two, each
    /// cell centred in that height. That is what lets a one-word label sit beside a cell framed
    /// on its own rather than above it.
    ///
    /// A row is cut and squared up as it arrives, because neither depends on anything but the row
    /// itself. What needs the whole table — how wide the columns come out — waits for [`draw`].
    pub fn push(&mut self, label: &str, value: &str) {
        self.rows.push(Row::cut(label, value).aligned());
    }

    /// The drawn lines, frame included, in order and without their line terminators.
    pub fn lines(&self) -> Vec<String> {
        draw(&self.title, &self.rows, &PADDING)
    }
}

/// The lines, joined by newlines and with none at the end — a block to hand to `println!`.
impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.lines().join("\n"))
    }
}

/// The blank room a table keeps around what its cells hold.
///
/// Every width and height elsewhere in this module is measured on content alone; this is what
/// turns those into a drawn size, and it is the only place a table's spacing is decided.
struct Padding {
    /// Blank columns between a cell and each border beside it.
    horizontal: usize,
    /// Blank lines between a row's cells and each rule above and below them.
    vertical: usize,
}

impl Padding {
    /// The columns the title has to itself: both content columns, the blanks that flank the
    /// divider, and the divider it spans over. Not the blanks at the two ends — the title has its
    /// own pair of those, which is why they are absent here.
    fn title_span(&self, columns: &Columns) -> usize {
        columns.label + columns.value + 2 * self.horizontal + DIVIDER_WIDTH
    }

    /// A content width once the blank the frame keeps on each side of it is counted in.
    fn around(&self, content: usize) -> usize {
        content + 2 * self.horizontal
    }
}

/// A row's two cells, each cut into the lines it occupies.
///
/// A cell's height needs no separate reckoning: it is the length of its own list.
struct Row {
    label: Vec<String>,
    value: Vec<String>,
}

impl Row {
    fn cut(label: &str, value: &str) -> Row {
        Row {
            label: cut_into_lines(label),
            value: cut_into_lines(value),
        }
    }

    /// The lines the row takes, which is what its taller cell takes.
    fn height(&self) -> usize {
        self.label.len().max(self.value.len())
    }

    /// The same row with both cells as tall as the row, each centred in that height.
    ///
    /// Heights only. Widths are not settled here because settling them means knowing how much
    /// blank the frame keeps beside a cell, and that is the frame's business — a cell leaves this
    /// step holding exactly what it holds.
    fn aligned(self) -> Row {
        let height = self.height();

        Row {
            label: centred(self.label, height),
            value: centred(self.value, height),
        }
    }
}

/// What each column measures: the content alone, with no blank around it.
struct Columns {
    label: usize,
    value: usize,
}

impl Columns {
    /// The width of the widest cell in each column.
    fn fitting(rows: &[Row]) -> Columns {
        Columns {
            label: widest(rows.iter().flat_map(|row| row.label.iter())),
            value: widest(rows.iter().flat_map(|row| row.value.iter())),
        }
    }

    /// The same columns, widened if the title asks for more room than they leave it.
    ///
    /// The value column takes the whole difference: a title is a caption for the table, not a
    /// reason to push its first column away from its own content.
    fn stretched_for(self, title: &str, padding: &Padding) -> Columns {
        let missing = display_width(title).saturating_sub(padding.title_span(&self));

        Columns {
            value: self.value + missing,
            ..self
        }
    }
}

/// The whole of what a table looks like, drawn around cells that hold their content and nothing
/// else.
///
/// Everything that depends on the decoration is here and only here: how wide the columns come
/// out, how much blank stands beside a cell and above a row, and the pieces of the frame. The
/// steps before this one measure and align content, and none of them has to know that a border
/// exists.
fn draw(title: &str, rows: &[Row], padding: &Padding) -> Vec<String> {
    let columns = Columns::fitting(rows).stretched_for(title, padding);
    let label_rule = rule(padding.around(columns.label));
    let value_rule = rule(padding.around(columns.value));
    let title_span = padding.title_span(&columns);

    let mut lines = vec![
        format!("{TOP_LEFT}{}{TOP_RIGHT}", rule(padding.around(title_span))),
        framed(&[(title, title_span)], padding),
        format!("{LEFT_TEE}{label_rule}{TOP_TEE}{value_rule}{RIGHT_TEE}"),
    ];
    lines.extend(rows.iter().flat_map(|row| draw_row(row, &columns, padding)));
    lines.push(format!("{BOTTOM_LEFT}{label_rule}{BOTTOM_TEE}{value_rule}{BOTTOM_RIGHT}"));

    lines
}

/// One row's lines, the vertical padding added above and below its cells.
fn draw_row(row: &Row, columns: &Columns, padding: &Padding) -> Vec<String> {
    let above_and_below = vec![String::new(); padding.vertical];
    let labels = above_and_below.iter().chain(row.label.iter()).chain(above_and_below.iter());
    let values = above_and_below.iter().chain(row.value.iter()).chain(above_and_below.iter());

    labels
        .zip(values)
        .map(|(label, value)| framed(&[(label, columns.label), (value, columns.value)], padding))
        .collect()
}

/// A line of cells, each grown to the width it is given and set between borders, with the blank
/// of `padding` on either side of it.
///
/// Both kinds of line in a table are this: the title is one cell spanning the width, a row is
/// two. Writing either by hand means counting blanks in a format string, and a miscount reads as
/// a table that leans.
fn framed(cells: &[(&str, usize)], padding: &Padding) -> String {
    let blank = " ".repeat(padding.horizontal);
    let divider = VERTICAL.to_string();
    let padded: Vec<String> = cells.iter().map(|(cell, width)| format!("{blank}{}{blank}", pad(cell, *width))).collect();

    format!("{VERTICAL}{}{VERTICAL}", padded.join(&divider))
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
pub fn boxed(content: &str) -> [String; 3] {
    let edge = rule(display_width(content));

    [
        format!("{TOP_LEFT}{edge}{TOP_RIGHT}"),
        format!("{VERTICAL}{content}{VERTICAL}"),
        format!("{BOTTOM_LEFT}{edge}{BOTTOM_RIGHT}"),
    ]
}

/// The lines of a cell, an empty cell counting as one blank line rather than none.
fn cut_into_lines(cell: &str) -> Vec<String> {
    let lines: Vec<String> = cell.lines().map(String::from).collect();

    if lines.is_empty() {
        vec![String::new()]
    }
    else {
        lines
    }
}

/// `cell` grown to `height` lines, its own sitting in the middle and the rest left empty.
///
/// Content in the middle and not at the top: a tall cell — a boxed gauge is three lines — should
/// carry its label beside its content rather than above it. Both columns are grown by this same
/// rule, which is why the label needs no rule of its own. When the blank lines do not divide
/// evenly the extra one goes below, so the content leans up, as a caption does.
fn centred(cell: Vec<String>, height: usize) -> Vec<String> {
    let above = (height - cell.len()) / 2;

    let mut lines = vec![String::new(); above];
    lines.extend(cell);
    lines.resize(height, String::new());

    lines
}

/// The widest of a set of lines, in columns.
fn widest<'a>(lines: impl Iterator<Item = &'a String>) -> usize {
    lines.map(|line| display_width(line)).max().unwrap_or(0)
}

/// A horizontal rule of `width` columns.
fn rule(width: usize) -> String {
    std::iter::repeat(HORIZONTAL).take(width).collect()
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
    /// apart from it — which is how the configuration round-trip test reads a table back, and
    /// what the drawing in this module's own documentation has to agree with.
    #[test]
    fn a_row_carries_two_cells_and_neither_the_title_nor_the_frame_does() {
        let lines = two_rows().lines();

        assert_eq!(4, lines[3].split(VERTICAL).count());
        assert_eq!(3, lines[1].split(VERTICAL).count());
        assert_eq!(1, lines[0].split(VERTICAL).count());
        // The top rule spans the whole table, so it carries no joint either.
        assert_eq!(1, lines[0].split(TOP_TEE).count());
        assert_eq!(2, lines[2].split(TOP_TEE).count());
    }

    /// Both columns are grown by the same rule, so a label of several lines is laid out exactly
    /// as a value of several lines is — and, unlike a label treated as one string, it does not
    /// carry a newline into the middle of a drawn line.
    #[test]
    fn a_multi_line_label_is_laid_out_like_a_multi_line_value() {
        let mut table = Table::new("A title");
        table.push("one\ntwo", "a\nb\nc");
        let lines = table.lines();
        let width = display_width(&lines[0]);

        assert_eq!(7, lines.len());
        assert!(lines[3].contains("one"));
        assert!(lines[4].contains("two"));
        for line in &lines {
            assert_eq!(width, display_width(line), "{line}");
        }
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

    /// Widths and heights are measured on content alone, and the blank around it comes from the
    /// padding and nowhere else — so changing the padding changes the drawn size and nothing
    /// about what the cells hold.
    #[test]
    fn the_padding_is_what_decides_the_drawn_size() {
        let tight = Padding { horizontal: 0, vertical: 0 };
        let loose = Padding { horizontal: 2, vertical: 1 };
        let rows = vec![Row::cut("label", "value").aligned()];

        let drawn = |padding: &Padding| draw("A title", &rows, padding);

        // Two blanks per side of two cells, so eight columns more than none at all.
        assert_eq!(display_width(&drawn(&tight)[0]) + 8, display_width(&drawn(&loose)[0]));
        // And one blank line above the row, one below.
        assert_eq!(drawn(&tight).len() + 2, drawn(&loose).len());
    }

    #[test]
    fn display_width_counts_columns_and_neither_bytes_nor_characters() {
        assert_eq!(5, display_width("value"));
        assert_eq!(5, display_width(&format!("{GREEN}value{RESET}")));
        // Three bytes, one column.
        assert_eq!(1, display_width("│"));
    }
}
