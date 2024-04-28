use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use clap::ArgMatches;
use regex::{Regex, RegexBuilder};

type MyResult<T> = Result<T, Box<dyn Error>>;

struct Input {
    name: String,
    file: File,
}

#[derive(Debug)]
struct Fortune {
    source: String,
    text: String
}

const ARG_SOURCE_ID: &str = "SOURCES";
const ARG_REGEX_ID: &str = "PATTERN";
const ARG_SEED_ID: &str = "SEED";
const ARG_INSENS_ID: &str = "INSENSITIVE";

#[derive(Debug)]
pub struct Config {
    sources: Vec<String>,
    pattern: Option<Regex>,
    seed: Option<u64>,
    buffer_size: usize,
    offset: usize,
}

pub fn get_args() -> MyResult<Config> {
    let mut args = clap::Command::new("fortuner")
        .author("MajinPierce")
        .version("0.1.0")
        .about("fortune but Rust")
        .arg(clap::Arg::new(ARG_SOURCE_ID)
            .num_args(1..)
            .required(true))
        .arg(clap::Arg::new(ARG_REGEX_ID)
            .short('m')
            .long("pattern"))
        .arg(clap::Arg::new(ARG_INSENS_ID)
            .short('i')
            .long("case-insensitive")
            .requires(ARG_REGEX_ID)
            .action(clap::ArgAction::SetTrue))
        .arg(clap::Arg::new(ARG_SEED_ID)
            .short('s')
            .long("seed"))
        .arg(clap::Arg::new("buffer")
            .short('l'))
        .arg(clap::Arg::new("offset")
            .short('o'))
        .get_matches();

    let sources = parse_file_names(&mut args)?;
    let pattern = build_pattern(&mut args)?;
    let seed = parse_seed(&mut args)?;
    let buffer_size: usize = args.remove_one::<String>("buffer").unwrap().parse()?;
    let offset: usize = args.remove_one::<String>("offset").unwrap().parse()?;
    Ok(Config{sources, pattern, seed, buffer_size, offset})
}

fn parse_file_names(args: &mut ArgMatches) -> MyResult<Vec<String>> {
    match args.remove_many::<String>(ARG_SOURCE_ID) {
        Some(sources) => Ok(sources.collect()),
        None => Err(From::from(String::from("Could not read file names"))),
    }
}

fn build_pattern(args: &mut ArgMatches) -> MyResult<Option<Regex>> {
    if !args.contains_id(ARG_REGEX_ID) {
        return Ok(None);
    }
    let mut pattern: String = args.remove_one(ARG_REGEX_ID).unwrap();
    let regex =  RegexBuilder::new(pattern.as_str())
        .case_insensitive(args.get_flag(ARG_INSENS_ID))
        .build()?;
    Ok(Some(regex))
}

fn parse_seed(args: &mut ArgMatches) -> MyResult<Option<u64>> {
    if !args.contains_id(ARG_SEED_ID) {
        return Ok(None);
    }
    let seed_str =  args.remove_one::<String>(ARG_SEED_ID).unwrap();
    let seed: u64 = seed_str.parse()?;
    Ok(Some(seed))
}

pub fn run(config: Config) -> MyResult<()> {
    let mut buf = vec![0u8; config.buffer_size];
    let mut input = open_file(config.sources.get(0).unwrap().as_str()).unwrap();
    input.file.seek(SeekFrom::Start(config.offset as u64))?;
    input.file.read_exact(&mut buf)?;
    let string = String::from_utf8_lossy(&buf);
    println!("{string}");
    Ok(())
}

fn open_file(path: &str) -> Option<Input> {
    match File::open(path) {
        Ok(open_file) => {
            Some(Input{
                name: String::from(path),
                file: open_file,
            })
        },
        Err(e) => {
            eprintln!("Could not open {path}: {e}");
            None
        }
    }
}