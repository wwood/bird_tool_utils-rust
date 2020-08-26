use std;
use std::env;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::process;

use clap::*;
use env_logger::Builder;
use log::LevelFilter;
use bird_tool_utils_man as man;
use bird_tool_utils_man::prelude::{Opt, Section};
use tempfile;

pub fn set_log_level(matches: &clap::ArgMatches, is_last: bool, program_name: &str, version: &str) {
    let mut log_level = LevelFilter::Info;
    let mut specified = false;
    if matches.is_present("verbose") {
        specified = true;
        log_level = LevelFilter::Debug;
    }
    if matches.is_present("quiet") {
        specified = true;
        log_level = LevelFilter::Error;
    }
    if specified || is_last {
        let mut builder = Builder::new();
        builder.filter_level(log_level);
        if env::var("RUST_LOG").is_ok() {
            builder.parse_filters(&env::var("RUST_LOG").unwrap());
        }
        if builder.try_init().is_err() {
            panic!("Failed to set log level - has it been specified multiple times?")
        }
    }
    if is_last {
        info!("{} version {}", program_name, version);
    }
}

pub fn print_full_help_if_needed(m: &clap::ArgMatches, manual: man::Manual) {
    if m.is_present("full-help") {
        display_full_help(manual)
    } else if m.is_present("full-help-roff") {
        println!("{}", manual.render());
        process::exit(1);
    }
}

/// Parse clap arguments defined in the common way, returning a list of paths as
/// strings. If fail_on_no_genomes, return an Err if no genomes were detected.
pub fn parse_list_of_genome_fasta_files(
    m: &clap::ArgMatches,
    fail_on_no_genomes: bool,
) -> std::result::Result<Vec<String>, String> {
    match m.is_present("genome-fasta-files") {
        true => {
            return Ok(m
                .values_of("genome-fasta-files")
                .unwrap()
                .map(|s| s.to_string())
                .collect())
        }
        false => {
            if m.is_present("genome-fasta-directory") {
                let dir = m.value_of("genome-fasta-directory").unwrap();
                let paths = std::fs::read_dir(dir)
                    .expect(&format!("Failed to read genome-fasta-directory '{}'", dir));
                let mut genome_fasta_files: Vec<String> = vec![];
                let extension = m.value_of("genome-fasta-extension").unwrap();
                for path in paths {
                    let file = path.unwrap().path();
                    match file.extension() {
                        Some(ext) => {
                            if ext == extension {
                                let s = String::from(file.to_string_lossy());
                                genome_fasta_files.push(s);
                            } else {
                                info!(
                                    "Not using directory entry '{}' as a genome FASTA file, as \
                                     it does not end with the extension '{}'",
                                    file.to_str().expect("UTF8 error in filename"),
                                    extension
                                );
                            }
                        }
                        None => {
                            info!(
                                "Not using directory entry '{}' as a genome FASTA file",
                                file.to_str().expect("UTF8 error in filename")
                            );
                        }
                    }
                }
                if genome_fasta_files.len() == 0 {
                    return match fail_on_no_genomes {
                        true => std::result::Result::Err(
                            "Found 0 genomes from the genome-fasta-directory, cannot continue."
                                .to_string(),
                        ),
                        false => Ok(vec![]),
                    };
                }
                return Ok(genome_fasta_files);
            } else if m.is_present("genome-fasta-list") {
                let file_path = m.value_of("genome-fasta-list").unwrap();
                let file = File::open(file_path).expect(&format!(
                    "Failed to open genome fasta list file {}",
                    file_path
                ));
                let reader = BufReader::new(file);
                let mut fasta_paths = vec![];
                for (index, line) in reader.lines().enumerate() {
                    let line = line.expect(&format!(
                        "Error when reading genome fasta list file {} on line {}",
                        file_path,
                        index + 1
                    ));
                    // Show the line and its number.
                    fasta_paths.push(line.trim().to_string());
                }
                return Ok(fasta_paths);
            } else {
                return std::result::Result::Err(
                    "No genome specification options specified".to_string(),
                );
            }
        }
    }
}

/// Add --genome-fasta-files and --genome-fasta-directory etc. to a clap App /
/// subcommand. These arguments can later be parsed with
/// parse_list_of_genome_fasta_files().
pub fn add_genome_specification_arguments<'a>(subcommand: clap::App<'a, 'a>) -> clap::App<'a, 'a> {
    subcommand
        .arg(
            Arg::with_name("genome-fasta-files")
                .short("f")
                .long("genome-fasta-files")
                .help("List of fasta files for processing")
                .multiple(true)
                .conflicts_with_all(&["genome-fasta-directory", "genome-fasta-list"])
                .takes_value(true),
        )
        .arg(
            Arg::with_name("genome-fasta-list")
                .long("genome-fasta-list")
                .help("List of fasta file paths, one per line, for processing")
                .conflicts_with("genome-fasta-directory")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("genome-fasta-directory")
                .long("genome-fasta-directory")
                .help("Directory containing fasta files for processing")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("genome-fasta-extension")
                .short("x")
                .help("File extension of FASTA files in --genome-fasta-directory")
                .long("genome-fasta-extension")
                // Unsure why, but uncommenting causes test failure (in
                // coverm genome mode where this code was pasted from,
                // not sure about here) - clap bug?
                //.requires("genome-fasta-directory")
                .default_value("fna")
                .takes_value(true),
        )
}

pub fn add_genome_specification_to_section(section: Section) -> Section {
    section
        .option(
            Opt::new("PATH ..")
                .short("-f")
                .long("--genome-fasta-files")
                .help(&format!(
                    "Path(s) to FASTA files of each genome e.g. {}.",
                    monospace_roff("pathA/genome1.fna pathB/genome2.fa")
                )),
        )
        .option(
            Opt::new("PATH")
                .short("-d")
                .long("--genome-fasta-directory")
                .help("Directory containing FASTA files of each genome."),
        )
        .option(
            Opt::new("EXT")
                .short("-x")
                .long("--genome-fasta-extension")
                .help(&format!(
                    "File extension of genomes in the directory \
                specified with {}. {}",
                    monospace_roff("-d/--genome-fasta-directory"),
                    default_roff("fna")
                )),
        )
        .option(
            Opt::new("PATH")
                .long("--genome-fasta-list")
                .help("File containing FASTA file paths, one per line."),
        )
}

pub fn display_full_help(manual: man::Manual) {
    let mut f =
        tempfile::NamedTempFile::new().expect("Failed to create temporary file for --full-help");
    write!(f, "{}", manual.render()).expect("Failed to write to tempfile for full-help");
    let child = std::process::Command::new("man")
        .args(&[f.path()])
        .spawn()
        .expect("Failed to spawn 'man' command for --full-help");

    crate::command::finish_command_safely(child, &"man");
    std::process::exit(1);
}

pub fn default_roff(s: &str) -> String {
    format!("[default: \\f[C]{}\\f[R]]", s)
}

pub fn monospace_roff(s: &str) -> String {
    format!("\\f[C]{}\\f[R]", s)
}

pub fn list_roff(strings: &[&str]) -> String {
    let mut s: String = "\n".to_string(); //start with a new line so the first .IP starts at the first char of the row
    for e in strings {
        s.push_str(".IP \\[bu] 2\n");
        s.push_str(e.clone());
        s.push_str("\n");
    }
    s.push_str(".PP\n");
    s
}

pub fn table_roff(strings: &[&[&str]]) -> String {
    //start with a new line so the first .IP starts at the first char of the row
    let mut s: String = "\n.TS\n\
        tab(@);\n"
        .to_string();
    for row in strings {
        for _ in *row {
            s.push_str("l ");
        }
        break;
    }
    s.push_str(".\n");

    let mut first_row = true;
    for e in strings {
        let mut first_column = true;
        for cell in *e {
            if first_column {
                first_column = false;
            } else {
                s.push_str("@");
            }
            s.push_str("T{\n");
            s.push_str(cell.clone());
            s.push_str("\nT}");
        }
        s.push_str("\n");
        if first_row {
            first_row = false;
            s.push_str("_\n");
        }
    }
    s.push_str(".TE\n");
    s
}
