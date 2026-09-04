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
    },
    OptionDesc {
        name: "--output",
        set: parse_output_filename,
    },
    OptionDesc {
        name: "--near",
        set: parse_near,
    },
    OptionDesc {
        name: "--far",
        set: parse_far,
    },
    OptionDesc {
        name: "--fov",
        set: parse_fov,
    },
    OptionDesc {
        name: "--output_width",
        set: parse_output_width,
    },
    OptionDesc {
        name: "--output_height",
        set: parse_output_height,
    },
    OptionDesc {
        name: "--max_depth",
        set: parse_max_depth,
    },
    OptionDesc {
        name: "--samples_ppx",
        set: parse_samples_ppx,
    },
    OptionDesc {
        name: "--threads",
        set: parse_threads,
    },
    OptionDesc {
        name: "--lens_radius",
        set: parse_lens_radius,
    },
    OptionDesc {
        name: "--focal_distance",
        set: parse_focal_distance,
    },
    OptionDesc {
        name: "--seed",
        set: parse_seed,
    },
    OptionDesc {
        name: "--integrator",
        set: parse_integrator,
    },
    OptionDesc {
        name: "--renderer",
        set: parse_renderer,
    },
];

fn find_option(name: &str) -> Option<&'static OptionDesc> {
    OPTIONS.iter().find(|desc| desc.name == name)
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

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "
        input_filename : {:?}\n
        output_filename : {:?}\n
        near : {:?}\n
        far : {:?}\n
        fov_deg : {:?}\n
        output_width : {:?}\n
        output_height : {:?}\n
        max_depth : {:?}\n
        samples_ppx : {:?}\n
        threads : {:?}\n
        lens_radius : {:?}\n
        focal_distance : {:?}\n
        seed : {:?}\n
        integrator : {:?}\n
        ",
            self.input_filename,
            self.output_filename,
            self.near,
            self.far,
            self.fov_deg,
            self.output_width,
            self.output_height,
            self.max_depth,
            self.samples_ppx,
            self.threads,
            self.lens_radius,
            self.focal_distance,
            self.seed,
            self.integrator
        )
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
