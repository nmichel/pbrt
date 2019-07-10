use std::collections::HashMap;

#[derive(Debug)]
pub struct Config {
    input_filename: String,
    output_filename: String
}

type OptionParserFn = fn(&mut Config, &String);

fn parse_input_filename(config: &mut Config, value: &String) {
    config.input_filename = value.clone();
}

fn parse_output_filename(config: &mut Config, value: &String) {
    config.output_filename = value.clone();
}

fn default_config() -> Config {
    Config {
        input_filename: "input".to_string(),
        output_filename: "output".to_string()
    }
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        let mut c = default_config();

        if args.len() <= 1 {
            return Err("not enough arguments")
        }

        let options_parsers: HashMap<&str, OptionParserFn> = [
            ("--input", parse_input_filename as OptionParserFn),
            ("--output", parse_output_filename as OptionParserFn)
        ].iter().cloned().collect();

        for o in &args[1..] {
            match options_parsers.get(&o[..]) {
                Some(&f) => {
                    println!("Found {:?}", o);
                    f(&mut c, &"prout".to_string());
                }
                None => {
                    println!("Unknown option {:?}", o);
                }
            }
        }

        Ok(c)
    }
}

