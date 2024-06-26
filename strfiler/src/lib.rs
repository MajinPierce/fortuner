use clap::ArgMatches;
use std::error::Error;
use std::fs::{metadata, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use walkdir::WalkDir;

type MyResult<T> = Result<T, Box<dyn Error>>;
type Input = Box<dyn BufRead>;

const ARG_FILES_ID: &str = "FILES";
const ARG_DELIM_ID: &str = "DELIMITER";
const MAX_FILE_SIZE_BYTES: u64 = 16777216;

#[derive(Debug)]
pub struct Config {
    sources: Vec<String>,
    delim: String,
}

struct Entry {
    offset: usize,
    size: usize,
}

pub fn get_args() -> MyResult<Config> {
    let args = clap::Command::new("strfiler")
        .author("MajinPierce")
        .version("0.1.0")
        .about("strfile but Rust")
        .arg(clap::Arg::new(ARG_FILES_ID).num_args(1..).required(true))
        .arg(
            clap::Arg::new(ARG_DELIM_ID)
                .short('c')
                .long("delim")
                .default_value("%"),
        )
        .get_matches();

    get_config_from_args(args)
}

fn get_config_from_args(mut args: ArgMatches) -> MyResult<Config> {
    let sources = parse_file_names(&mut args)?;
    let delim = args.remove_one(ARG_DELIM_ID).unwrap();
    Ok(Config { sources, delim })
}

fn parse_file_names(args: &mut ArgMatches) -> MyResult<Vec<String>> {
    if let Some(files) = args.remove_many::<String>(ARG_FILES_ID) {
        Ok(files.collect())
    } else {
        Err(From::from("Could not read file names"))
    }
}

pub fn run(config: Config) -> MyResult<()> {
    let sources = get_full_source_list(&config)?;
    for source in sources {
        create_dat_for_source(&source, &config.delim)?;
    }
    Ok(())
}

fn get_full_source_list(config: &Config) -> MyResult<Vec<String>> {
    let mut sources: Vec<String> = Vec::new();
    for path in &config.sources {
        match metadata(path) {
            Ok(meta) => {
                if meta.is_dir() {
                    sources.append(&mut read_dir(path));
                } else {
                    sources.push(path.clone());
                }
            }
            Err(e) => {
                eprintln!("{path}: {e}");
            }
        }
    }

    if sources.is_empty() {
        Err(From::from("No valid sources. Please check files."))
    } else {
        Ok(sources)
    }
}

fn read_dir(dir: &str) -> Vec<String> {
    WalkDir::new(dir)
        .into_iter()
        .map(|result| String::from(result.unwrap().path().to_str().unwrap()))
        .filter(|sub_path| !metadata(sub_path).unwrap().is_dir())
        .filter(|sub_path| !is_invalid_type(sub_path))
        .collect()
}

fn is_invalid_type(path: &str) -> bool {
    let path = Path::new(path);
    let is_dat = match path.extension() {
        None => false,
        Some(ext) => ext == "dat",
    };
    let is_invalid = path.file_name().unwrap() == ".DS_Store";

    is_dat || is_invalid
}

fn create_dat_for_source(source: &str, delim: &str) -> MyResult<()> {
    eprintln!("creating dat for {source}");
    if let Some(mut input) = open_file(source) {
        let entries = get_entry_locations(&mut input, delim)?;
        if !entries.is_empty() {
            write_entries(source, entries);
        } else {
            eprintln!("source is empty: {source}");
        }
    };
    Ok(())
}

fn open_file(path: &str) -> Option<Input> {
    match File::open(path) {
        Ok(open_file) => {
            if open_file.metadata().unwrap().len() >= MAX_FILE_SIZE_BYTES {
                eprintln!("File too large: {path}");
                return None;
            }
            Some(Box::new(BufReader::new(open_file)))
        }
        Err(e) => {
            eprintln!("Could not open {path}: {e}");
            None
        }
    }
}

fn get_entry_locations(input: &mut Input, delim: &str) -> MyResult<Vec<Entry>> {
    let mut offset: usize = 0;
    let mut entry_size: usize = 0;
    let mut buf = String::new();
    let mut entries: Vec<Entry> = Vec::new();
    loop {
        let line_size = input.read_line(&mut buf)?;
        if line_size == 0 {
            if !entries.is_empty() {
                entries.push(Entry {
                    offset,
                    size: entry_size,
                });
            }
            break;
        }
        if buf.as_str().trim() == delim {
            entries.push(Entry {
                offset,
                size: entry_size,
            });
            offset += entry_size + buf.len();
            entry_size = 0;
            buf.clear();
            continue;
        }
        entry_size += line_size;
        buf.clear();
    }
    let entries = filter_empty_entries(entries);
    Ok(entries)
}

fn filter_empty_entries(entries: Vec<Entry>) -> Vec<Entry> {
    entries.into_iter().filter(|entry| entry.size > 0).collect()
}

fn write_entries(file_name: &str, entries: Vec<Entry>) {
    let dat_file = Path::new(file_name).with_extension("dat");
    let mut file = File::create(dat_file).unwrap();
    let size = format!("{:0>6x}\n", entries.len());
    file.write_all(size.as_bytes()).unwrap();
    for entry in entries {
        let formatted_entry = format!("{:0>6x} {:0>6x}\n", entry.offset, entry.size);
        file.write_all(formatted_entry.as_bytes()).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_get_full_source_list() {
        let source_dir = String::from("./tests/inputs");
        let config = Config {
            sources: vec![source_dir],
            delim: "%".to_string(),
        };
        let expected_sources_str = vec![
            "./tests/inputs/ascii-art",
            "./tests/inputs/jokes",
            "./tests/inputs/literature",
            "./tests/inputs/quotes",
            "./tests/inputs/empty/.gitkeep",
        ];
        let expected_sources: Vec<String> = expected_sources_str
            .iter()
            .map(|source| String::from(*source))
            .collect();
        let result = get_full_source_list(&config);

        assert!(result.is_ok());
        let sources = result.unwrap();
        for source in sources.iter() {
            assert!(expected_sources.iter().any(|s| s == source));
        }
    }

    #[test]
    fn test_get_full_source_list_empty() {
        let source_dir = String::from("./tests/inputs/empty_dir");
        let config = Config {
            sources: vec![source_dir],
            delim: "%".to_string(),
        };
        let result = get_full_source_list(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_invalid_type_no_extension() {
        let invalid = is_invalid_type("tests/ascii-art");
        assert!(!invalid)
    }

    #[test]
    fn test_is_invalid_type_txt() {
        let invalid = is_invalid_type("tests/ascii-art.txt");
        assert!(!invalid)
    }

    #[test]
    fn test_is_invalid_type_dat() {
        let invalid = is_invalid_type("tests/ascii-art.dat");
        assert!(invalid)
    }

    #[test]
    fn test_is_invalid_type_ds_store() {
        let invalid = is_invalid_type("tests/.DS_Store");
        assert!(invalid)
    }

    #[test]
    fn test_get_entry_locations() {
        let cursor = io::Cursor::new(b"lorem\n%\nipsum\n%\ndolor\n");
        let mut buf_read: Input = Box::new(cursor);

        let result = get_entry_locations(&mut buf_read, "%");

        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(3, entries.len());
        //first entry
        assert_eq!(0, entries.get(0).unwrap().offset);
        assert_eq!(6, entries.get(0).unwrap().size);
        //second entry
        assert_eq!(8, entries.get(1).unwrap().offset);
        assert_eq!(6, entries.get(1).unwrap().size);
        //third entry
        assert_eq!(16, entries.get(2).unwrap().offset);
        assert_eq!(6, entries.get(2).unwrap().size);
    }
}
