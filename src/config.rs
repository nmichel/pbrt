use std::collections::HashMap;
use std::slice::ChunksExact;

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

        let pairs_iter: ChunksExact<_> = args[1..].chunks_exact(2);
        if pairs_iter.remainder().len() > 0 {
            let rem = &pairs_iter.remainder()[0];
            println!("found trailing parameter {:?}", &rem);
            return Err("trailing parameter")
        }

        let pairs: Vec<_> = pairs_iter.collect(); // _ stands for &[String]
        for pair in pairs {
            let k = &pair[0];
            match options_parsers.get(&k[..]) {
                Some(&f) => {
                    println!("Found {:?}", k);
                    let v = &pair[1];
                    f(&mut c, &v.to_string());
                }
                None => {
                    println!("Unknown option {:?}", k);
                    return Err("Unknown option")
                }
            }
        }

        Ok(c)
    }
}

