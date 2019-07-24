use std::collections::HashMap;
use std::slice::ChunksExact;
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
}

type OptionParserFn = fn(&mut Config, &String);

fn parse_input_filename(config: &mut Config, value: &String) {
    config.input_filename = value.clone();
}

fn parse_output_filename(config: &mut Config, value: &String) {
    config.output_filename = value.clone();
}

fn parse_near(config: &mut Config, value: &String) {
    config.near = f64::from_str(value).unwrap();
}

fn parse_far(config: &mut Config, value: &String) {
    config.far = f64::from_str(value).unwrap();
}

fn parse_fov(config: &mut Config, value: &String) {
    config.fov_deg = f64::from_str(value).unwrap();
}

fn parse_output_width(config: &mut Config, value: &String) {
    config.output_width = usize::from_str(value).unwrap();
}

fn parse_output_height(config: &mut Config, value: &String) {
    config.output_height = usize::from_str(value).unwrap();
}

fn parse_max_depth(config: &mut Config, value: &String) {
    config.max_depth = usize::from_str(value).unwrap();
}

fn default_config() -> Config {
    Config {
        input_filename: "input".to_string(),
        output_filename: "output.png".to_string(),
        near: 0.01,
        far: 1000.0,
        fov_deg: 90.0,
        output_width: 1920,
        output_height: 1280,
        max_depth: 3
    }
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        let mut c = default_config();

        let options_parsers: HashMap<&str, OptionParserFn> = [
            ("--input", parse_input_filename as OptionParserFn),
            ("--output", parse_output_filename as OptionParserFn),
            ("--near", parse_near as OptionParserFn),
            ("--far", parse_far as OptionParserFn),
            ("--fov", parse_fov as OptionParserFn),
            ("--output_width", parse_output_width as OptionParserFn),
            ("--output_height", parse_output_height as OptionParserFn),
            ("--max_depth", parse_max_depth as OptionParserFn)
        ].iter().cloned().collect();

        let pairs_iter: ChunksExact<_> = args[1..].chunks_exact(2);
        if pairs_iter.remainder().len() > 0 {
            let rem = &pairs_iter.remainder()[0];
            println!("found trailing parameter {:?}", &rem);
            return Err("trailing parameter")
        }

        let pairs: Vec<_> = pairs_iter.collect(); // _ stands for &[String]
        for pair in pairs {
            let k = &pair[0];
            println!("Found {:?}", k);
            let f = options_parsers.get(&k[..]).expect("Unknown option");
            let v = &pair[1];
            f(&mut c, &v.to_string());
        }

        Ok(c)
    }
}

