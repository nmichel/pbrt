use crate::{integrators, renderers};
use std::error::Error;
use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct Config {
    pub input_filename: String,
    pub output_filename: String,
    pub near: f64,
    pub far: f64,
    pub fov_deg: f64,
    pub output_width: usize,
    pub output_height: usize,
    pub max_depth: usize,
    pub samples_ppx: usize,
    pub threads: usize,
    pub lens_radius: f64,
    pub focal_distance: f64,

    /// Chooses which set of paths a render traces.
    ///
    /// It is an *option* and not an internal constant, because the promise is "same scene, same
    /// options, same image": two seeds give two renders, each repeatable. That is how one tells a
    /// noisy speckle from a bug once two runs have become bit-identical, and how one measures the
    /// spread of an estimator, which a single realisation cannot give.
    /// See `docs/rendu_reproductible.md` §5.
    pub seed: u64,

    pub integrator: integrators::Type,
    pub renderer: renderers::Type,
}

/// Everything that keeps a command line from producing a [`Config`].
///
/// Every variant names the option at fault. A message that only says "bad argument" leaves the
/// user to bisect their own command line, and a run that starts on a misread option is worse
/// still: `--treads 8` rendering single-threaded looks like a slow machine, not like a typo.
#[derive(Debug, PartialEq)]
pub enum ConfigError {
    /// A token where an option name was expected.
    UnknownOption(String),
    /// A known option, but the command line ends before its value.
    MissingValue(&'static str),
    /// A value the option cannot make sense of.
    InvalidValue {
        option: &'static str,
        value: String,
        expected: &'static str,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::UnknownOption(token) => write!(f, "unknown option {:?}", token),
            ConfigError::MissingValue(option) => write!(f, "{} expects a value, none given", option),
            ConfigError::InvalidValue { option, value, expected } => write!(f, "{} does not accept {:?}, expected {}", option, value, expected),
        }
    }
}

impl Error for ConfigError {}

// What an option wants, phrased as the user has to type it. `FromStr` failures carry either
// `()` or a message about the parser, and neither says what to write instead, so the
// expectation is stated here and handed to the error.
const A_NUMBER: &str = "a number";
const A_POSITIVE_INTEGER: &str = "a positive integer";
const A_NON_NEGATIVE_INTEGER: &str = "a non-negative integer";

/// One command line option: its name, and how to read it into a [`Config`].
struct OptionDesc {
    /// The name as written on the command line, `--` included.
    name: &'static str,
    /// Reads `value` into the matching field. It takes the name back so that a rejected value
    /// can name the option it was handed to.
    set: fn(&mut Config, &'static str, &str) -> Result<(), ConfigError>,
    /// Writes the matching field back out, as the value this option would accept.
    show: fn(&Config) -> String,
    /// One line of help, for `usage`.
    help: &'static str,
}

/// Reads `value` as a `T`, turning a refusal into the error that names the option.
fn value_of<T: FromStr>(option: &'static str, value: &str, expected: &'static str) -> Result<T, ConfigError> {
    T::from_str(value).map_err(|_| {
        ConfigError::InvalidValue {
            option,
            value: value.to_string(),
            expected,
        }
    })
}

/// Reads `value` as a count, which zero is not.
///
/// Zero satisfies `usize` and only fails further on — no thread to render on, no sample to
/// average with, or an image with no pixel. Refusing it here names the option that is wrong,
/// where a division by zero deep inside a renderer names nothing.
fn count_of(option: &'static str, value: &str) -> Result<usize, ConfigError> {
    let count: usize = value_of(option, value, A_POSITIVE_INTEGER)?;
    if count == 0 {
        return Err(ConfigError::InvalidValue {
            option,
            value: value.to_string(),
            expected: A_POSITIVE_INTEGER,
        });
    }
    Ok(count)
}

fn parse_input_filename(config: &mut Config, _option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.input_filename = value.to_string();
    Ok(())
}

fn parse_output_filename(config: &mut Config, _option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.output_filename = value.to_string();
    Ok(())
}

fn parse_near(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.near = value_of(option, value, A_NUMBER)?;
    Ok(())
}

fn parse_far(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.far = value_of(option, value, A_NUMBER)?;
    Ok(())
}

fn parse_fov(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.fov_deg = value_of(option, value, A_NUMBER)?;
    Ok(())
}

fn parse_output_width(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.output_width = count_of(option, value)?;
    Ok(())
}

fn parse_output_height(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.output_height = count_of(option, value)?;
    Ok(())
}

/// Zero is a legitimate depth — the run that gathers emission and nothing else — so this one is
/// not a `count_of`.
fn parse_max_depth(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.max_depth = value_of(option, value, A_NON_NEGATIVE_INTEGER)?;
    Ok(())
}

fn parse_samples_ppx(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.samples_ppx = count_of(option, value)?;
    Ok(())
}

fn parse_threads(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.threads = count_of(option, value)?;
    Ok(())
}

fn parse_lens_radius(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.lens_radius = value_of(option, value, A_NUMBER)?;
    Ok(())
}

fn parse_focal_distance(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.focal_distance = value_of(option, value, A_NUMBER)?;
    Ok(())
}

/// Every seed is a legitimate seed, zero included: it names a stream, it does not size anything.
fn parse_seed(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.seed = value_of(option, value, A_NON_NEGATIVE_INTEGER)?;
    Ok(())
}

fn parse_integrator(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.integrator = value_of(option, value, integrators::Type::ACCEPTED_VALUES)?;
    Ok(())
}

fn parse_renderer(config: &mut Config, option: &'static str, value: &str) -> Result<(), ConfigError> {
    config.renderer = value_of(option, value, renderers::Type::ACCEPTED_VALUES)?;
    Ok(())
}

/// The known options, and the single place that knows them.
///
/// An array rather than a map: fifteen string comparisons at start-up cost nothing, and the
/// order is the one thing a map would take away.
static OPTIONS: [OptionDesc; 15] = [
    OptionDesc {
        name: "--input",
        set: parse_input_filename,
        show: |config| config.input_filename.clone(),
        help: "scene to render, in the .stage language",
    },
    OptionDesc {
        name: "--output",
        set: parse_output_filename,
        show: |config| config.output_filename.clone(),
        help: "PNG file the image is written to",
    },
    OptionDesc {
        name: "--near",
        set: parse_near,
        show: |config| config.near.to_string(),
        help: "distance below which an intersection is ignored",
    },
    OptionDesc {
        name: "--far",
        set: parse_far,
        show: |config| config.far.to_string(),
        help: "distance beyond which a ray hits nothing",
    },
    OptionDesc {
        name: "--fov",
        set: parse_fov,
        show: |config| config.fov_deg.to_string(),
        help: "field of view, in degrees",
    },
    OptionDesc {
        name: "--output_width",
        set: parse_output_width,
        show: |config| config.output_width.to_string(),
        help: "image width, in pixels",
    },
    OptionDesc {
        name: "--output_height",
        set: parse_output_height,
        show: |config| config.output_height.to_string(),
        help: "image height, in pixels",
    },
    OptionDesc {
        name: "--max_depth",
        set: parse_max_depth,
        show: |config| config.max_depth.to_string(),
        help: "longest path traced, in bounces",
    },
    OptionDesc {
        name: "--samples_ppx",
        set: parse_samples_ppx,
        show: |config| config.samples_ppx.to_string(),
        help: "paths traced per pixel",
    },
    OptionDesc {
        name: "--threads",
        set: parse_threads,
        show: |config| config.threads.to_string(),
        help: "workers the mt renderer starts",
    },
    OptionDesc {
        name: "--lens_radius",
        set: parse_lens_radius,
        show: |config| config.lens_radius.to_string(),
        help: "lens radius; zero keeps the whole scene sharp",
    },
    OptionDesc {
        name: "--focal_distance",
        set: parse_focal_distance,
        show: |config| config.focal_distance.to_string(),
        help: "distance the lens is focused on",
    },
    OptionDesc {
        name: "--seed",
        set: parse_seed,
        show: |config| config.seed.to_string(),
        help: "chooses which set of paths a render traces",
    },
    OptionDesc {
        name: "--integrator",
        set: parse_integrator,
        show: |config| config.integrator.to_string(),
        help: "how light transport is solved",
    },
    OptionDesc {
        name: "--renderer",
        set: parse_renderer,
        show: |config| config.renderer.to_string(),
        help: "how the pixel loop is driven",
    },
];

fn find_option(name: &str) -> Option<&'static OptionDesc> {
    OPTIONS.iter().find(|desc| desc.name == name)
}

/// The name that asks for the option list instead of for a render.
///
/// It is not one of [`OPTIONS`]: it takes no value, and it yields no configuration — it replaces
/// the run rather than describing it. What a process does about that is the caller's business,
/// which is why `new` does not know the name and `usage` only prints it.
pub const HELP_OPTION: &str = "--help";

/// Whether `args` asks for the option list.
///
/// Anywhere on the line and the answer is yes, including after an option that would otherwise be
/// refused: someone who writes `--treads 8 --help` is asking precisely because they are unsure of
/// a name.
pub fn help_requested(args: &[String]) -> bool {
    args.iter().any(|token| token == HELP_OPTION)
}

/// The option list: for each one, its name, what it sets, and what it is worth when the command
/// line stays silent about it.
///
/// The defaults are not spelled out here. They are read off [`default_config`] through the very
/// `show` that prints a configuration, so a default that changes cannot go on being advertised
/// as what it used to be.
pub fn usage() -> String {
    let defaults = default_config();
    let name_column = OPTIONS.iter().map(|desc| desc.name.len()).max().unwrap_or(0).max(HELP_OPTION.len());
    let help_column = OPTIONS.iter().map(|desc| desc.help.len()).max().unwrap_or(0);

    let mut rows = vec![
        "usage: pbrt [--option value]...".to_string(),
        String::new(),
        format!("  {:<name_column$}  {:<help_column$}  [default]", "option", "what it sets"),
    ];
    for desc in OPTIONS.iter() {
        rows.push(format!(
            "  {:<name_column$}  {:<help_column$}  [{}]",
            desc.name,
            desc.help,
            (desc.show)(&defaults)
        ));
    }
    rows.push(format!("  {:<name_column$}  {}", HELP_OPTION, "print this list and render nothing"));

    rows.join("\n")
}

/// The settings a run uses when the command line says nothing.
///
/// Public because a second binary needs them: `bvh_stats` loads a `.stage` scene through
/// `Loader::load_scene`, which takes a `Config` to build the camera from. Measuring the rays the
/// renderer would actually cast means starting from the renderer's own defaults rather than
/// inventing a parallel set.
pub fn default_config() -> Config {
    Config {
        input_filename: "input".to_string(),
        output_filename: "output.png".to_string(),
        near: 0.0001,
        far: 1000.0,
        fov_deg: 90.0,
        output_width: 800,
        output_height: 600,
        max_depth: 3,
        samples_ppx: 5,
        threads: 1,
        lens_radius: 0.0,
        focal_distance: 1.0,
        seed: 0,
        integrator: integrators::Type::PATH,
        renderer: renderers::Type::MT,
    }
}

impl Config {
    /// Builds a configuration from the process arguments, `args[0]` included and ignored.
    ///
    /// Options come in pairs `--name value`, and every departure from that shape is an error:
    /// an unreadable command line stops the run rather than starting one that quietly differs
    /// from what was asked. A repeated option keeps its last value, so that a wrapping script
    /// can override what it passes on.
    ///
    /// Nothing here is printed. Reporting belongs to the caller, which is what lets a test read
    /// a command line in silence.
    pub fn new(args: &[String]) -> Result<Config, ConfigError> {
        let mut config = default_config();
        let mut tokens = args.iter().skip(1);

        while let Some(token) = tokens.next() {
            let desc = find_option(token).ok_or_else(|| ConfigError::UnknownOption(token.clone()))?;
            let value = tokens.next().ok_or(ConfigError::MissingValue(desc.name))?;
            (desc.set)(&mut config, desc.name, value)?;
        }

        Ok(config)
    }
}

/// The title the framed table carries, above every row.
const TABLE_TITLE: &str = "Rendering configuration";

/// Draws the configuration as a framed table: a title spanning the whole width, then one row per
/// option — the name a command line would use, and the value that command line would give it.
///
/// It walks [`OPTIONS`], the same list `new` reads from, which is what makes the two agree by
/// construction: a row cannot go missing without the option itself going missing. The values are
/// written as the options accept them, `path` and not `PATH`, so that the rows read back as a
/// command line reproducing the run.
impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let values: Vec<String> = OPTIONS.iter().map(|desc| (desc.show)(self)).collect();

        // Each column is as wide as its widest cell, so no width is fixed once and left to rot:
        // a longer option name or a longer path widens the frame instead of breaking it. Widths
        // count characters and not bytes, which a path outside ASCII would inflate.
        let name_column = OPTIONS.iter().map(|desc| desc.name.chars().count()).max().unwrap_or(0);
        let widest_value = values.iter().map(|value| value.chars().count()).max().unwrap_or(0);
        // The title spans both columns and the divider between them. A title wider than that
        // stretches the value column rather than overflowing the frame.
        let value_column = widest_value.max(TABLE_TITLE.chars().count().saturating_sub(name_column + 3));
        let title_span = name_column + value_column + 3;

        let name_rule = "─".repeat(name_column + 2);
        let value_rule = "─".repeat(value_column + 2);

        writeln!(f, "┌{}┐", "─".repeat(title_span + 2))?;
        writeln!(f, "│ {TABLE_TITLE:<title_span$} │")?;
        writeln!(f, "├{name_rule}┬{value_rule}┤")?;
        for (desc, value) in OPTIONS.iter().zip(values.iter()) {
            writeln!(f, "│ {:<name_column$} │ {:<value_column$} │", desc.name, value)?;
        }
        write!(f, "└{name_rule}┴{value_rule}┘")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A command line as `env::args` hands it over, program name in front.
    fn command_line(args: &[&str]) -> Vec<String> {
        std::iter::once("pbrt").chain(args.iter().copied()).map(String::from).collect()
    }

    #[test]
    fn an_empty_command_line_yields_the_defaults() {
        let config = Config::new(&command_line(&[])).unwrap();

        assert_eq!(default_config().output_width, config.output_width);
        assert_eq!(default_config().samples_ppx, config.samples_ppx);
        assert_eq!(default_config().integrator, config.integrator);
    }

    #[test]
    fn every_option_reaches_its_field() {
        let config = Config::new(&command_line(&[
            "--input",
            "scene.stage",
            "--output",
            "image.png",
            "--near",
            "0.5",
            "--far",
            "2000",
            "--fov",
            "45",
            "--output_width",
            "1280",
            "--output_height",
            "720",
            "--max_depth",
            "10",
            "--samples_ppx",
            "1000",
            "--threads",
            "8",
            "--lens_radius",
            "0.25",
            "--focal_distance",
            "3.5",
            "--seed",
            "42",
            "--integrator",
            "naive",
            "--renderer",
            "st",
        ]))
        .unwrap();

        assert_eq!("scene.stage", config.input_filename);
        assert_eq!("image.png", config.output_filename);
        assert_eq!(0.5, config.near);
        assert_eq!(2000.0, config.far);
        assert_eq!(45.0, config.fov_deg);
        assert_eq!(1280, config.output_width);
        assert_eq!(720, config.output_height);
        assert_eq!(10, config.max_depth);
        assert_eq!(1000, config.samples_ppx);
        assert_eq!(8, config.threads);
        assert_eq!(0.25, config.lens_radius);
        assert_eq!(3.5, config.focal_distance);
        assert_eq!(42, config.seed);
        assert_eq!(integrators::Type::NAIVE, config.integrator);
        assert_eq!(renderers::Type::ST, config.renderer);
    }

    #[test]
    fn a_repeated_option_keeps_its_last_value() {
        let config = Config::new(&command_line(&["--threads", "4", "--threads", "8"])).unwrap();

        assert_eq!(8, config.threads);
    }

    /// The table is a command line, so feeding it back yields the configuration it came from.
    /// This is what one list serving both parsing and printing buys, and the test fails the
    /// moment a row and an option disagree.
    ///
    /// An option row is the only line carrying two cells, so the frame and the title sort
    /// themselves out of the way.
    #[test]
    fn the_printed_table_reads_back_as_the_configuration_it_shows() {
        let config = Config::new(&command_line(&["--samples_ppx", "64", "--integrator", "normal", "--renderer", "st"])).unwrap();
        let table = config.to_string();

        let mut arguments: Vec<&str> = Vec::new();
        for row in table.lines() {
            let cells: Vec<&str> = row.split('│').collect();
            if cells.len() != 4 {
                continue;
            }
            arguments.push(cells[1].trim());
            arguments.push(cells[2].trim());
        }
        let read_back = Config::new(&command_line(&arguments)).unwrap();

        assert_eq!(OPTIONS.len() * 2, arguments.len());
        assert_eq!(table, read_back.to_string());
    }

    /// Same list, third use: an option missing from the help is an option that does not exist.
    #[test]
    fn the_option_list_mentions_every_option() {
        let usage = usage();

        for desc in OPTIONS.iter() {
            assert!(usage.contains(desc.name), "{} is missing from the option list", desc.name);
        }
        assert!(usage.contains(HELP_OPTION));
    }

    #[test]
    fn help_is_seen_even_next_to_a_line_that_would_be_refused() {
        assert!(help_requested(&command_line(&["--treads", "8", "--help"])));
        assert!(!help_requested(&command_line(&["--threads", "8"])));
    }

    #[test]
    fn a_misspelled_option_is_refused() {
        let error = Config::new(&command_line(&["--treads", "8"])).unwrap_err();

        assert_eq!(ConfigError::UnknownOption("--treads".to_string()), error);
    }

    /// A value where an option name belongs — a forgotten `--input`, most often.
    #[test]
    fn a_stray_token_is_refused() {
        let error = Config::new(&command_line(&["scene.stage"])).unwrap_err();

        assert_eq!(ConfigError::UnknownOption("scene.stage".to_string()), error);
    }

    #[test]
    fn an_option_without_its_value_is_refused() {
        let error = Config::new(&command_line(&["--threads", "8", "--samples_ppx"])).unwrap_err();

        assert_eq!(ConfigError::MissingValue("--samples_ppx"), error);
    }

    #[test]
    fn a_value_of_the_wrong_kind_is_refused() {
        let error = Config::new(&command_line(&["--threads", "many"])).unwrap_err();

        assert_eq!(
            ConfigError::InvalidValue {
                option: "--threads",
                value: "many".to_string(),
                expected: A_POSITIVE_INTEGER,
            },
            error
        );
    }

    #[test]
    fn a_count_of_zero_is_refused() {
        assert!(Config::new(&command_line(&["--threads", "0"])).is_err());
        assert!(Config::new(&command_line(&["--samples_ppx", "0"])).is_err());
        assert!(Config::new(&command_line(&["--output_width", "0"])).is_err());
    }

    /// A depth of zero gathers emission and stops, which is a legitimate run.
    #[test]
    fn a_depth_of_zero_is_accepted() {
        let config = Config::new(&command_line(&["--max_depth", "0"])).unwrap();

        assert_eq!(0, config.max_depth);
    }

    #[test]
    fn an_unknown_integrator_is_refused_rather_than_replaced_by_a_default() {
        let error = Config::new(&command_line(&["--integrator", "quantum"])).unwrap_err();

        assert_eq!(
            ConfigError::InvalidValue {
                option: "--integrator",
                value: "quantum".to_string(),
                expected: integrators::Type::ACCEPTED_VALUES,
            },
            error
        );
    }

    #[test]
    fn an_unknown_renderer_is_refused_rather_than_replaced_by_a_default() {
        let error = Config::new(&command_line(&["--renderer", "gpu"])).unwrap_err();

        assert_eq!(
            ConfigError::InvalidValue {
                option: "--renderer",
                value: "gpu".to_string(),
                expected: renderers::Type::ACCEPTED_VALUES,
            },
            error
        );
    }
}
