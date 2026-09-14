use crate::table::{boxed, display_width, Table};
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// The table's title, and the label of each of its rows.
const TITLE: &str = "Rendering progress";
const GAUGE_LABEL: &str = "progress";
const ELAPSED_LABEL: &str = "elapsed";
const REMAINING_LABEL: &str = "remaining";
const TOTAL_LABEL: &str = "total";

/// Cells the gauge is wide, borders excluded. Forty is enough for every percent to move the fill
/// at least every third step, and short enough to leave a terminal room for the rest.
const GAUGE_WIDTH: usize = 40;

/// A filled cell and an empty one.
const FILLED_CELL: char = '█';
const EMPTY_CELL: char = ' ';

/// SGR escapes, from ECMA-48 §8.3.117: `32` sets the foreground to green, `0` puts every
/// attribute back the way it was. The reset closes the sequence before the gauge's border, so a
/// line cut short leaves no colour behind.
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

/// Digits held for the percentage, so that `1` and `100` end in the same column.
const PERCENT_DIGITS: usize = 3;

/// The three lines [`boxed`] hands back, named so that indexing one of them says which.
const TOP_LINE: usize = 0;
const TRACK_LINE: usize = 1;
const BOTTOM_LINE: usize = 2;

const SECONDS_PER_MINUTE: u64 = 60;
const MINUTES_PER_HOUR: u64 = 60;
const MILLIS_PER_SECOND: u128 = 1_000;

/// The share of the work below which no forecast is offered.
///
/// Under one percent the extrapolation is carried by the handful of steps that happen to have
/// run: one pixel of a million announces three hours, and the figure swings by minutes from one
/// redraw to the next. One percent of a 1280×720 image is nine thousand pixels, enough for the
/// average to settle into something worth reading.
const FORECAST_FROM_PERCENT: usize = 1;

/// What a duration reads as while it cannot be forecast. As wide as an `HH:MM:SS`, so that the
/// frame does not breathe at the moment the forecast appears.
const UNKNOWN_DURATION: &str = "--:--:--";

/// A table redrawn in place on the terminal while a render runs, holding a gauge and the time
/// spent so far.
///
/// ```text
/// ┌───────────┬─────────────────────────────────────────────────┐
/// │ Rendering progress                                          │
/// ├───────────┬─────────────────────────────────────────────────┤
/// │           │ ┌────────────────────────────────────────┐      │
/// │ progress  │ │███████████████████                     │  48% │
/// │           │ └────────────────────────────────────────┘      │
/// │ elapsed   │ 00:01:23                                        │
/// │ remaining │ 00:01:29                                        │
/// │ total     │ 00:02:52                                        │
/// └───────────┴─────────────────────────────────────────────────┘
/// ```
///
/// Three things shape what follows:
///
/// - **Rewriting a block takes more than a carriage return.** `\r` returns to the start of the
///   line it is on and nothing more, so the table moves the cursor up over the lines it last drew
///   and writes them again. Every line ends with a newline, which is also why `finish` has none
///   to add: the cursor already sits below the frame.
/// - **The buffer has to be pushed.** `Stdout` is a `LineWriter`, and it does empty itself on the
///   newlines this table writes — but only line by line, so a redraw would reach the terminal in
///   pieces. Each redraw is built as one string, written once, and flushed.
/// - **A redraw is not free.** A render calls [`ProgressReport::advance`] once per pixel — close
///   to a million times for a 1280×720 image. The table redraws only when the fill, the
///   percentage or the second would come out different, which is at most a hundred and twenty-one
///   times plus once a second.
pub struct ProgressReport {
    /// Steps that make the work complete. Never zero, so that a share can be divided by it.
    total: usize,
    /// Steps done so far.
    done: usize,
    /// When the work started, which is what the elapsed row counts from.
    started: Instant,
    /// Fill, percentage and second as they currently stand on screen, or `None` before the first
    /// draw. This is what a fresh state is compared against to decide on a redraw.
    drawn: Option<(usize, usize, u64)>,
    /// Lines the last draw put on screen, and therefore how far up the cursor must go to write
    /// over them.
    lines_drawn: usize,
}

impl ProgressReport {
    /// A report for a job of `total` steps, drawn at once so the table exists before the first
    /// step lands — a render that takes a minute to reach one percent still shows that it started,
    /// and its clock runs.
    pub fn new(total: usize) -> Self {
        // A job of no steps is complete from the outset; one step keeps the division defined and
        // leaves the gauge full, which is what "nothing left to do" looks like.
        let mut report = ProgressReport {
            total: total.max(1),
            done: 0,
            started: Instant::now(),
            drawn: None,
            lines_drawn: 0,
        };
        report.redraw();
        report
    }

    /// Records one more step done, and redraws if that changes what the table shows.
    pub fn advance(&mut self) {
        self.done += 1;
        self.redraw();
    }

    /// Draws the finished state, whatever the last redraw left on screen.
    pub fn finish(&mut self) {
        self.done = self.total;
        // Forgetting what is on screen forces the draw: a job whose last step landed inside the
        // same second as the one before would otherwise stop short of its own completion.
        self.drawn = None;
        self.redraw();
    }

    fn redraw(&mut self) {
        let share = self.done.min(self.total);
        let filled = share * GAUGE_WIDTH / self.total;
        let percent = share * 100 / self.total;
        let so_far = self.started.elapsed();
        let seconds = so_far.as_secs();

        // The forecast rows are a function of the share done and the time it took, and both are
        // in this key already — so they cannot go stale between two redraws without one of these
        // three moving first, and the cadence is the one the gauge alone would have.
        if self.drawn == Some((filled, percent, seconds)) {
            return;
        }
        self.drawn = Some((filled, percent, seconds));

        let lines = table(&gauge(filled, percent), &Durations::at(share, self.total, so_far)).lines();

        // One string, one write, one flush: a line at a time would show the table being built.
        let mut frame = String::new();
        if self.lines_drawn > 0 {
            frame.push_str(&cursor_up(self.lines_drawn));
        }
        for line in &lines {
            // The carriage return costs nothing and holds even if something else left the cursor
            // mid-line.
            frame.push('\r');
            frame.push_str(line);
            frame.push('\n');
        }
        self.lines_drawn = lines.len();

        print!("{frame}");

        // A failed flush is a closed or full stdout, and there is nothing to say about it that
        // could be said anywhere but stdout. The render is not the poorer for it.
        let _ = io::stdout().flush();
    }
}

/// Moves the cursor up `lines` lines, leaving the column where it is: `ESC [ n A`, the CUU of
/// ECMA-48 §8.3.22. This is what a carriage return cannot do, and what rewriting a block needs.
fn cursor_up(lines: usize) -> String {
    format!("\x1b[{lines}A")
}

/// The gauge cell: a green fill growing inside a frame of its own, and the percentage to its
/// right.
///
/// Three lines, since the frame is a box and not a pair of uprights. The percentage rides on the
/// middle one, outside the box: the box marks where the track begins and ends — that is what
/// tells a fill of a third from a fill of two — and a number inside it would move that mark.
fn gauge(filled: usize, percent: usize) -> String {
    let track: String = std::iter::repeat(FILLED_CELL)
        .take(filled)
        .chain(std::iter::repeat(EMPTY_CELL).take(GAUGE_WIDTH - filled))
        .collect();

    let frame = boxed(&format!("{GREEN}{track}{RESET}"));

    format!(
        "{}\n{} {percent:>PERCENT_DIGITS$}%\n{}",
        frame[TOP_LINE], frame[TRACK_LINE], frame[BOTTOM_LINE]
    )
}

/// The three durations a report shows: what the render has taken, what is left of it, and what it
/// is heading for in all.
///
/// They are held together because two of the three are read off the same extrapolation, and
/// because three cells of identical shape passed one after another are three chances to hand the
/// table the wrong one.
struct Durations {
    elapsed: String,
    remaining: String,
    total: String,
}

impl Durations {
    /// The three cells after `so_far`, with `share` of the job's `steps` done.
    ///
    /// The forecast assumes **the steps left cost what the steps done have cost on average**, so
    /// the whole job is heading for `so_far × steps / share`. That assumption is false in the
    /// small: pixels are handed out in scan order, and a band of empty background does not cost
    /// what the mesh below it costs, so the figure drifts while the render crosses an expensive
    /// region and comes back as it leaves. Averaging over the whole run rather than over the last
    /// steps is what keeps it from chasing that relief — a sliding window would follow it and
    /// swing harder.
    ///
    /// `steps` is never zero: [`ProgressReport::new`] sees to that.
    fn at(share: usize, steps: usize, so_far: Duration) -> Durations {
        let elapsed = so_far.as_secs();

        if share * 100 / steps < FORECAST_FROM_PERCENT {
            return Durations {
                elapsed: clock(elapsed),
                remaining: UNKNOWN_DURATION.to_string(),
                total: UNKNOWN_DURATION.to_string(),
            };
        }

        // Milliseconds rather than seconds, because a job whose first percent passes in under a
        // second would otherwise forecast a total of zero. The product is taken in `u128`, which
        // leaves it no room to overflow at any scale a render reaches.
        let total = (so_far.as_millis() * steps as u128 / share as u128 / MILLIS_PER_SECOND) as u64;

        Durations {
            elapsed: clock(elapsed),
            // The two agree at the end: at the last step the forecast is the elapsed time itself,
            // so the table closes on a remaining of zero rather than on a stale guess.
            remaining: clock(total.saturating_sub(elapsed)),
            total: clock(total),
        }
    }
}

/// A duration cell, as `HH:MM:SS`.
///
/// Fixed fields rather than the shortest form that fits, so that the cell does not change width
/// from one second to the next and the frame does not breathe. Hours are not capped, and a render
/// that runs past a hundred of them widens the cell by a character — which changes nothing, since
/// the value column is already as wide as the gauge.
fn clock(seconds: u64) -> String {
    let hours = seconds / (SECONDS_PER_MINUTE * MINUTES_PER_HOUR);
    let minutes = (seconds / SECONDS_PER_MINUTE) % MINUTES_PER_HOUR;
    let seconds = seconds % SECONDS_PER_MINUTE;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn table(gauge: &str, durations: &Durations) -> Table {
    let mut table = Table::new(TITLE);
    table.push(GAUGE_LABEL, gauge);
    table.push(ELAPSED_LABEL, &durations.elapsed);
    table.push(REMAINING_LABEL, &durations.remaining);
    table.push(TOTAL_LABEL, &durations.total);
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gauge as it would be drawn at `done` steps out of `total`, one string per line.
    fn gauge_at(done: usize, total: usize) -> Vec<String> {
        gauge(done * GAUGE_WIDTH / total, done * 100 / total).lines().map(String::from).collect()
    }

    /// One percent and a hundred end in the same column, which is the whole point of holding three
    /// digits for a number that needs one.
    ///
    /// The measure is in columns: the frame and the filled cell take three bytes each and the
    /// colour escapes take none, so neither bytes nor characters would call two lined-up gauges
    /// the same length.
    #[test]
    fn the_percentage_is_right_aligned() {
        let one = &gauge_at(1, 100)[TRACK_LINE];
        let hundred = &gauge_at(100, 100)[TRACK_LINE];

        assert_eq!(display_width(one), display_width(hundred));
        assert!(one.ends_with("   1%"));
        assert!(hundred.ends_with(" 100%"));
    }

    /// The box stands still while the fill grows inside it: the track always measures the whole
    /// job, never the part of it that is done. Its top and bottom never move at all, which is what
    /// makes the fill readable at a glance.
    #[test]
    fn the_box_delimits_the_whole_gauge() {
        let empty = gauge_at(0, 100);

        for done in 0..=100 {
            let drawn = gauge_at(done, 100);

            assert_eq!(3, drawn.len());
            assert_eq!(empty[TOP_LINE], drawn[TOP_LINE]);
            assert_eq!(empty[BOTTOM_LINE], drawn[BOTTOM_LINE]);
            // The track is as wide as the rule its box is drawn with, percentage aside.
            assert_eq!(display_width(&drawn[TOP_LINE]), GAUGE_WIDTH + 2);
        }
    }

    #[test]
    fn the_fill_grows_with_the_work_and_fills_the_gauge_at_the_end() {
        assert_eq!(0, gauge_at(0, 100)[TRACK_LINE].matches(FILLED_CELL).count());
        assert_eq!(GAUGE_WIDTH / 2, gauge_at(50, 100)[TRACK_LINE].matches(FILLED_CELL).count());
        assert_eq!(GAUGE_WIDTH, gauge_at(100, 100)[TRACK_LINE].matches(FILLED_CELL).count());
    }

    /// The redraw is what costs, and a job of a million steps has barely more than a hundred
    /// distinct tables to show, the clock aside. Counting the states is counting the writes.
    ///
    /// A hundred and twenty-one, and not a hundred and one: the percentage steps at every
    /// hundredth of the job and the fill at every fortieth, and only twenty of those moments
    /// coincide. Either alone would redraw less often, but a gauge that stalls while the number
    /// climbs — or the reverse — is a gauge that looks stuck.
    #[test]
    fn a_step_that_changes_nothing_in_the_table_is_not_redrawn() {
        let total = 1_000_000;
        let frozen_clock = 0;
        let mut redraws = 1; // the empty table `new` draws
        let mut drawn = (0, 0, frozen_clock);

        for done in 1..=total {
            let state = (done * GAUGE_WIDTH / total, done * 100 / total, frozen_clock);
            if state != drawn {
                drawn = state;
                redraws += 1;
            }
        }

        assert_eq!(121, redraws);
    }

    /// The clock is the second thing that moves the table, and it is why the render does not look
    /// stuck between two percents.
    #[test]
    fn a_duration_reads_as_hours_minutes_and_seconds() {
        assert_eq!("00:00:00", clock(0));
        assert_eq!("00:00:59", clock(59));
        assert_eq!("00:01:00", clock(60));
        assert_eq!("01:00:00", clock(3600));
        assert_eq!("12:34:56", clock(12 * 3600 + 34 * 60 + 56));
    }

    /// Every second of a render draws the same width, so the frame does not breathe.
    #[test]
    fn a_duration_cell_keeps_its_width() {
        let widths: Vec<usize> = (0..3600 * 4).map(|seconds| clock(seconds).chars().count()).collect();

        assert!(widths.iter().all(|width| *width == widths[0]));
    }

    /// The forecast extrapolates: half the job in a minute is a job of two minutes, one of which
    /// is left.
    #[test]
    fn the_forecast_reads_the_rest_of_the_job_off_the_part_already_done() {
        let durations = Durations::at(50, 100, Duration::from_secs(60));

        assert_eq!("00:01:00", durations.elapsed);
        assert_eq!("00:01:00", durations.remaining);
        assert_eq!("00:02:00", durations.total);
    }

    /// What `finish` leaves on screen: nothing left to do, and a total that is the render's own
    /// duration rather than a guess at it.
    #[test]
    fn a_finished_job_forecasts_the_time_it_actually_took() {
        let durations = Durations::at(100, 100, Duration::from_secs(95));

        assert_eq!("00:01:35", durations.elapsed);
        assert_eq!("00:00:00", durations.remaining);
        assert_eq!(durations.elapsed, durations.total);
    }

    /// Below the threshold the extrapolation is carried by a handful of steps and says nothing,
    /// so the table says nothing either — but the time actually spent is never in doubt.
    #[test]
    fn no_forecast_is_offered_before_the_job_is_worth_extrapolating() {
        let durations = Durations::at(1, 1_000_000, Duration::from_secs(3));

        assert_eq!("00:00:03", durations.elapsed);
        assert_eq!(UNKNOWN_DURATION, durations.remaining);
        assert_eq!(UNKNOWN_DURATION, durations.total);

        // One percent of the same job is where the forecast starts.
        let durations = Durations::at(10_000, 1_000_000, Duration::from_secs(3));

        assert_eq!("00:05:00", durations.total);
    }

    /// The forecast appears mid-render, so the cell it lands in must be the width the placeholder
    /// held — a table that widens under way would leave the old frame behind it.
    #[test]
    fn the_placeholder_is_as_wide_as_a_duration() {
        assert_eq!(display_width(&clock(0)), display_width(UNKNOWN_DURATION));
    }

    /// A forecast in milliseconds and not in seconds: a job whose first percent passes in under a
    /// second still forecasts a duration rather than zero.
    #[test]
    fn a_fast_first_percent_still_forecasts_a_duration() {
        let durations = Durations::at(1, 100, Duration::from_millis(600));

        assert_eq!("00:01:00", durations.total);
    }

    /// The table as a half-done render draws it.
    fn half_drawn() -> Vec<String> {
        table(&gauge(20, 50), &Durations::at(50, 100, Duration::from_secs(83))).lines()
    }

    /// The table is redrawn over itself, so the cursor must climb exactly as far as the last draw
    /// went down. Ten lines: its own frame and title, the three the boxed gauge takes, and one
    /// for each of the three durations.
    #[test]
    fn the_table_is_the_height_the_cursor_climbs_back() {
        let lines = half_drawn();

        assert_eq!(10, lines.len());
        assert_eq!("\x1b[10A", cursor_up(lines.len()));
    }

    /// The gauge is three lines tall inside a cell, and the table stays a table: every line of it
    /// still comes out the same width, colour escapes and inner frame included.
    #[test]
    fn a_boxed_gauge_does_not_break_the_table() {
        let lines = half_drawn();
        let width = display_width(&lines[0]);

        for line in &lines {
            assert_eq!(width, display_width(line), "{line}");
        }
    }
}
