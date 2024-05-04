use std::error::Error;
use std::fs::{File, metadata};
use std::io::{BufRead, BufReader, Lines, Read, Seek, SeekFrom};
use std::path::Path;
use clap::ArgMatches;
use rand::prelude::{IteratorRandom, StdRng};
use rand::{Rng, SeedableRng};
use regex::{Regex, RegexBuilder};
use walkdir::WalkDir;

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
            .num_args(0..)
            .default_value("fortuner/tests/inputs")
        )
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
        None => Err(From::from("Could not read file names")),
    }
}

fn build_pattern(args: &mut ArgMatches) -> MyResult<Option<Regex>> {
    if !args.contains_id(ARG_REGEX_ID) {
        return Ok(None);
    }
    let pattern: String = args.remove_one(ARG_REGEX_ID).unwrap();
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
    let sources = get_full_source_list(&config)?;
    if config.pattern.is_some() {
        find_fortunes_matching_pattern(sources, config)
    } else {
        get_random_fortune(sources, config)
    }
}

fn get_full_source_list(config: &Config) -> MyResult<Vec<String>> {
    let mut sources: Vec<String> = Vec::new();
    for path in &config.sources {
        match metadata(path) {
            Ok(meta) => if meta.is_dir() {
                sources.append(&mut read_dir(path));
            } else {
                sources.push(path.clone())
            },
            Err(e) => {
                eprintln!("{path}: {e}");
            }
        }
    }
    sources = sources.into_iter()
        .filter(|path| has_dat(path))
        .collect();

    if sources.is_empty() {
        Err(From::from("No valid sources. Please check dat files."))
    } else {
        Ok(sources)
    }
}

fn get_random_fortune(sources: Vec<String>, config: Config) -> MyResult<()> {
    let file_name = pick_random_source(sources, &config.seed);
    let entry = pick_random_entry(&file_name, &config)?;
    let mut file = open_file(&entry.file_name)?;
    let fortune = read_fortune_from_file(&entry, &mut file)?;
    println!("{fortune}");
    Ok(())
}

fn find_fortunes_matching_pattern(sources: Vec<String>, config: Config) -> MyResult<()> {
    let mut all_entries: Vec<Vec<Entry>> = Vec::new();
    for source in &sources {
        let entries = get_entries(source)?;
        all_entries.push(entries);
    }
    find_matching_entries(all_entries, &config.pattern.unwrap())
}

fn read_dir(dir: &str) -> Vec<String> {
    WalkDir::new(dir).into_iter()
        .map(|result| String::from(result.unwrap().path().to_str().unwrap()))
        .filter(|sub_path| !metadata(sub_path).unwrap().is_dir())
        .filter(|sub_path| !is_dat(sub_path))
        .collect()
}

fn is_dat(path: &str) -> bool {
    match Path::new(path).extension() {
        None => false,
        Some(ext) => ext == "dat",
    }
}

fn has_dat(path: &str) -> bool {
    Path::new(path).with_extension("dat").exists()
}

fn pick_random_source(sources: Vec<String>, seed_opt: &Option<u64>) -> String {
    let mut rng = get_rng(seed_opt);
    let source = sources.iter()
        .choose(&mut rng)
        .expect("List of sources was empty");
    source.clone()
}

fn get_rng(seed_opt: &Option<u64>) -> StdRng {
    match seed_opt {
        None => StdRng::from_rng(rand::thread_rng()).unwrap(),
        Some(seed) => StdRng::seed_from_u64(*seed)
    }
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

fn get_entries(file_name: &str) -> MyResult<Vec<Entry>> {
    let dat_file = open_file_dat(file_name)?;
    let mut lines = dat_file.lines();
    let num_entries = parse_num_entries(&mut lines)?;
    let mut entries = Vec::new();
    for _ in 0..num_entries {
        let line = lines.next().unwrap()?;
        let line_elem: Vec<&str> = line.split_whitespace().collect();
        if line_elem.len() < 2 {
            return Err(From::from("dat file entry missing required info. Please validate dat file."));
        }
        let offset = usize::from_str_radix(line_elem.get(0).unwrap(), 16)?;
        let size = usize::from_str_radix(line_elem.get(1).unwrap(), 16)?;
        entries.push(Entry{file_name: String::from(file_name), offset, size})
    }
    Ok(entries)
}

fn parse_num_entries(lines: &mut Lines<BufReader<File>>) -> MyResult<usize> {
    let first_line = match lines.next() {
        None => return Err(From::from("dat file is blank")),
        Some(line) => line?
    };
    let num_entries = usize::from_str_radix(&first_line, 16)?;
    Ok(num_entries)
}

fn choose_entry_index(num_entries: usize, seed_opt: &Option<u64>) -> usize {
    let mut rng = get_rng(seed_opt);
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

fn find_matching_entries(all_entries: Vec<Vec<Entry>>, pattern: &Regex) -> MyResult<()> {
    for entries in all_entries {
        let mut file = open_file(&entries.get(0).unwrap().file_name)?;
        for entry in entries {
            let fortune = read_fortune_from_file(&entry, &mut file)?;
            if pattern.is_match(&fortune) {
                println!("{fortune}");
            }
        }
    }
    Ok(())
}

fn read_fortune_from_file(entry: &Entry, file: &mut File) -> MyResult<String> {
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