use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines, Read, Seek, SeekFrom};
use std::path::Path;
use clap::ArgMatches;
use rand::prelude::IteratorRandom;
use rand::{Rng, SeedableRng};
use regex::{Regex, RegexBuilder};

type MyResult<T> = Result<T, Box<dyn Error>>;

struct Entry {
    file_name: String,
    offset: usize,
    size: usize,
}

pub struct Config {
    sources: Vec<String>,
    pattern: Option<Regex>,
    seed: Option<u64>,
}

const ARG_SOURCE_ID: &str = "SOURCES";
const ARG_REGEX_ID: &str = "PATTERN";
const ARG_SEED_ID: &str = "SEED";
const ARG_INSENS_ID: &str = "INSENSITIVE";

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
        .get_matches();

    let sources = parse_file_names(&mut args)?;
    let pattern = build_pattern(&mut args)?;
    let seed = parse_seed(&mut args)?;
    Ok(Config{sources, pattern, seed})
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
    let file_name = pick_random_source(&config);
    let entry = pick_random_entry(file_name, &config)?;
    let fortune = read_fortune_from_file(entry)?;
    println!("{fortune}");
    Ok(())
}

fn pick_random_source(config: &Config) -> &String {
    let mut rng = match config.seed {
        None => rand::rngs::StdRng::from_rng(rand::thread_rng()).unwrap(),
        Some(seed) => rand::rngs::StdRng::seed_from_u64(seed)
    };
    let source = config.sources.iter()
        .choose(&mut rng)
        .expect("List of sources was empty");
    source
}

fn pick_random_entry(file_name: &str, config: &Config) -> MyResult<Entry> {
    let dat_file = open_file_dat(file_name)?;
    let mut lines = dat_file.lines();
    let num_entries = parse_num_entries(&mut lines)?;
    let chosen_entry_index = choose_entry_index(num_entries, &config.seed);
    let (offset, size) = parse_entry_offset_and_size(lines, chosen_entry_index)?;
    let entry = Entry{file_name: String::from(file_name), offset, size};
    Ok(entry)
}

fn parse_num_entries(mut lines: &mut Lines<BufReader<File>>) -> MyResult<usize> {
    let first_line = match lines.next() {
        None => return Err(From::from("dat file is blank")),
        Some(line) => line?
    };
    let num_entries = usize::from_str_radix(&first_line, 16)?;
    Ok(num_entries)
}

fn choose_entry_index(num_entries: usize, seed_opt: &Option<u64>) -> usize {
    let mut rng = match seed_opt {
        None => rand::rngs::StdRng::from_rng(rand::thread_rng()).unwrap(),
        Some(seed) => rand::rngs::StdRng::seed_from_u64(*seed)
    };
    rng.gen_range(0..num_entries)
}

fn parse_entry_offset_and_size(mut lines: Lines<BufReader<File>>, index: usize) -> MyResult<(usize, usize)>{
    let line = lines.nth(index).unwrap()?;
    let line_elem: Vec<&str> = line.split_whitespace().collect();
    if line_elem.len() < 2 {
        return Err(From::from("dat file entry missing required info. Please validate dat file."));
    }
    let offset = usize::from_str_radix(line_elem.get(0).unwrap(), 16)?;
    let size = usize::from_str_radix(line_elem.get(1).unwrap(), 16)?;
    Ok((offset, size))
}

fn read_fortune_from_file(entry: Entry) -> MyResult<String> {
    let mut file = open_file(&entry.file_name)?;
    let mut buf = vec![0u8; entry.size];
    file.seek(SeekFrom::Start(entry.offset as u64))?;
    file.read_exact(&mut buf)?;
    let fortune = String::from_utf8_lossy(&buf).into_owned();
    Ok(fortune)
}

fn open_file(path: &str) -> MyResult<File> {
    match File::open(path) {
        Ok(open_file) => Ok(open_file),
        Err(e) => Err(Box::from(e))
    }
}

fn open_file_dat(parent_path: &str) -> MyResult<BufReader<File>> {
    let dat_path = Path::new(parent_path).with_extension("dat");
    let file = File::open(dat_path)?;
    Ok(BufReader::new(file))
}