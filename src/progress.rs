use std::io::{self, Write};

/// Cells the gauge is wide, borders excluded. Forty is enough for every percent to move the
/// fill at least every third step, and short enough to leave a terminal room for the rest.
const GAUGE_WIDTH: usize = 40;

/// The borders. They stand where the gauge starts and where it ends, so the track shows how much
/// work is left without the empty part having to be drawn at all.
const LEFT_BORDER: char = '│';
const RIGHT_BORDER: char = '│';

/// A filled cell and an empty one.
const FILLED_CELL: char = '█';
const EMPTY_CELL: char = ' ';

/// SGR escapes, from ECMA-48 §8.3.117: `32` sets the foreground to green, `0` puts every
/// attribute back the way it was. The reset closes the sequence before the border, so a terminal
/// that stops reading mid-line — or a line cut short by a panic — leaves no colour behind.
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

/// Digits held for the percentage, so that `1` and `100` end in the same column.
const PERCENT_DIGITS: usize = 3;

/// A one-line gauge, drawn on the terminal and rewritten in place as work advances.
///
/// Each redraw returns to the start of the line with a carriage return and writes no newline, so
/// the gauge stays on one line until [`ProgressBar::finish`] closes it. Two consequences shape
/// what follows:
///
/// - **`\r` does not flush.** `Stdout` is a `LineWriter`, which empties itself on `\n` and on
///   nothing else, so a gauge that never writes one would sit in the buffer and reach the
///   terminal in bursts of a hundred already-stale redraws. Every redraw flushes explicitly.
/// - **A redraw is not free.** It takes the lock on stdout and writes the whole line, where a
///   render calls [`ProgressBar::advance`] once per pixel — close to a million times for a
///   1280×720 image, for the hundred distinct percentages a gauge can show. The bar redraws only
///   when what it would draw differs from what is already on the line.
pub struct ProgressBar {
    /// Steps that make the work complete. Never zero, so that a share can be divided by it.
    total: usize,
    /// Steps done so far.
    done: usize,
    /// Filled cells and percentage as they currently stand on the line, or `None` before the
    /// first draw. This is what a fresh share is compared against to decide on a redraw.
    drawn: Option<(usize, usize)>,
}

impl ProgressBar {
    /// A gauge for a job of `total` steps, drawn empty at once so the line exists before the
    /// first step lands — a render that takes a minute to reach one percent still shows that it
    /// started.
    pub fn new(total: usize) -> Self {
        // A job of no steps is complete from the outset; one step keeps the division defined and
        // leaves the gauge full, which is what "nothing left to do" looks like.
        let mut bar = ProgressBar {
            total: total.max(1),
            done: 0,
            drawn: None,
        };
        bar.redraw();
        bar
    }

    /// Records one more step done, and redraws if that changes what the line shows.
    pub fn advance(&mut self) {
        self.done += 1;
        self.redraw();
    }

    /// Closes the line with the newline no redraw ever wrote, so that what comes next — a shell
    /// prompt included — starts on a line of its own.
    pub fn finish(&mut self) {
        self.done = self.total;
        self.redraw();
        println!();
    }

    fn redraw(&mut self) {
        let share = self.done.min(self.total);
        let filled = share * GAUGE_WIDTH / self.total;
        let percent = share * 100 / self.total;

        if self.drawn == Some((filled, percent)) {
            return;
        }
        self.drawn = Some((filled, percent));

        let gauge: String = std::iter::repeat(FILLED_CELL)
            .take(filled)
            .chain(std::iter::repeat(EMPTY_CELL).take(GAUGE_WIDTH - filled))
            .collect();

        print!("\r{LEFT_BORDER}{GREEN}{gauge}{RESET}{RIGHT_BORDER} {percent:>PERCENT_DIGITS$}%");

        // A failed flush is a closed or full stdout, and there is nothing to say about it that
        // could be said anywhere but stdout. The render is not the poorer for it.
        let _ = io::stdout().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The line a gauge would draw at `done` steps out of `total`, escapes stripped.
    fn line(done: usize, total: usize) -> String {
        let filled = done * GAUGE_WIDTH / total;
        let gauge: String = std::iter::repeat(FILLED_CELL)
            .take(filled)
            .chain(std::iter::repeat(EMPTY_CELL).take(GAUGE_WIDTH - filled))
            .collect();
        format!(
            "{}{}{} {:>width$}%",
            LEFT_BORDER,
            gauge,
            RIGHT_BORDER,
            done * 100 / total,
            width = PERCENT_DIGITS
        )
    }

    /// One percent and a hundred end in the same column, which is the whole point of holding
    /// three digits for a number that needs one.
    ///
    /// Columns are characters and not bytes: the border and the filled cell take three bytes
    /// each, so a byte count would call the two lines different lengths when they line up.
    #[test]
    fn the_percentage_is_right_aligned() {
        assert_eq!(line(1, 100).chars().count(), line(100, 100).chars().count());
        assert!(line(1, 100).ends_with("   1%"));
        assert!(line(100, 100).ends_with(" 100%"));
    }

    /// The borders stand still while the fill grows between them: the track always measures the
    /// whole job, never the part of it that is done.
    #[test]
    fn the_borders_delimit_the_whole_gauge() {
        for done in 0..=100 {
            let line = line(done, 100);
            let cells: Vec<char> = line.chars().collect();

            assert_eq!(LEFT_BORDER, cells[0]);
            assert_eq!(RIGHT_BORDER, cells[GAUGE_WIDTH + 1]);
        }
    }

    #[test]
    fn the_fill_grows_with_the_work_and_fills_the_gauge_at_the_end() {
        assert_eq!(0, line(0, 100).matches(FILLED_CELL).count());
        assert_eq!(GAUGE_WIDTH / 2, line(50, 100).matches(FILLED_CELL).count());
        assert_eq!(GAUGE_WIDTH, line(100, 100).matches(FILLED_CELL).count());
    }

    /// The redraw is what costs, and a job of a million steps has barely more than a hundred
    /// distinct lines to show. Counting the states is counting the writes the gauge would make.
    ///
    /// A hundred and twenty-one, and not a hundred and one: the percentage steps at every
    /// hundredth of the job and the fill at every fortieth, and only twenty of those moments
    /// coincide. Either alone would redraw less often, but a gauge that stalls while the number
    /// climbs — or the reverse — is a gauge that looks stuck.
    #[test]
    fn a_step_that_changes_nothing_on_the_line_is_not_redrawn() {
        let total = 1_000_000;
        let mut redraws = 1; // the empty gauge `new` draws
        let mut drawn = (0, 0);

        for done in 1..=total {
            let state = (done * GAUGE_WIDTH / total, done * 100 / total);
            if state != drawn {
                drawn = state;
                redraws += 1;
            }
        }

        assert_eq!(121, redraws);
    }
}
