use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use clap::ArgMatches;

type MyResult<T> = Result<T, Box<dyn Error>>;
type Input = Box<dyn BufRead>;

const ARG_FILES_ID: &str = "FILES";
const ARG_DELIM_ID: &str = "DELIMITER";

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    delim: String,
}

pub fn get_args() -> MyResult<Config> {
    let args = clap::Command::new("strfiler")
        .author("MajinPierce")
        .version("0.1.0")
        .about("strfile but Rust")
        .arg(clap::Arg::new(ARG_FILES_ID)
            .num_args(1..)
            .required(true))
        .arg(clap::Arg::new(ARG_DELIM_ID)
            .short('c')
            .long("delim")
            .default_value("%"))
        .get_matches();

    get_config_from_args(args)
}

fn get_config_from_args(mut args: ArgMatches) -> MyResult<Config> {
    let files = parse_file_names(&mut args)?;
    let delim = args.remove_one(ARG_DELIM_ID).unwrap();
    Ok(Config{files, delim})
}

fn parse_file_names(args: &mut ArgMatches) -> MyResult<Vec<String>> {
    if let Some(files) = args.remove_many::<String>(ARG_FILES_ID) {
        Ok(files.collect())
    } else {
        Err(From::from(String::from("Could not read file names")))
    }
}

pub fn run(config: Config) -> MyResult<()> {
    for file in config.files {
        let mut input = open_file(file.as_str()).unwrap();
        get_file_entries(&mut input, config.delim.as_str())?;
    }
    Ok(())
}

fn open_file(path: &str) -> Option<Input> {
    match File::open(path) {
        Ok(open_file) => {
            Some(Box::new(BufReader::new(open_file)))
        },
        Err(e) => {
            eprintln!("Could not open {path}: {e}");
            None
        }
    }
}

fn get_file_entries(input: &mut Input, delim: &str) -> MyResult<()> {
    let mut offset: usize = 0;
    let mut entry_size: usize = 0;
    let mut buf = String::new();
    loop {
        let line_size = input.read_line(&mut buf)?;
        if line_size == 0 {
            println!("entry: offset {offset}, size {entry_size}");
            break;
        }
        if buf.as_str().trim() == delim {
            println!("entry: offset {offset}, size {entry_size}");
            offset += entry_size + buf.len();
            entry_size = 0;
            buf.clear();
            continue;
        }
        entry_size += line_size;
        buf.clear();
    }
    Ok(())
}