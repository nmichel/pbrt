use std::fmt;
use std::str::FromStr;

pub mod mt;
pub mod st;

/// Which renderer drives the pixel loop.
///
/// The mapping between a variant and the name the command line uses lives here rather than in
/// the caller that reads the option: adding a renderer is then a change to this module alone.
#[derive(Debug, PartialEq)]
pub enum Type {
    ST,
    MT,
}

impl Type {
    /// The accepted names, phrased for whoever mistyped one.
    pub const ACCEPTED_VALUES: &'static str = "one of st, mt";
}

/// Writes the name the command line uses, so a printed configuration reads back as a command
/// line that reproduces it — `to_string` and [`FromStr`] are inverse, which the doc-test below
/// checks.
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Type::ST => "st",
            Type::MT => "mt",
        };
        write!(f, "{}", name)
    }
}

/// ```
/// use pbrt::renderers::Type;
/// use std::str::FromStr;
///
/// for name in ["st", "mt"].iter() {
///     assert_eq!(*name, Type::from_str(name).unwrap().to_string());
/// }
/// assert!(Type::from_str("gpu").is_err());
/// ```
impl FromStr for Type {
    /// Nothing to carry: the message a user reads is built by the caller, which knows the
    /// option name the value was given to. [`Type::ACCEPTED_VALUES`] supplies the rest.
    type Err = ();

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "st" => Ok(Type::ST),
            "mt" => Ok(Type::MT),
            _ => Err(()),
        }
    }
}
