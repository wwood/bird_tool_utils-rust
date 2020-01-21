use std;
use std::env;

use clap::*;
use log::LevelFilter;
use env_logger::Builder;

pub fn set_log_level(matches: &clap::ArgMatches, is_last: bool, program_name: &str) {
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
        info!("{} version {}", program_name, crate_version!());
    }
}

/// Parse clap arguments defined in the common way, returning a list of paths as
/// strings. If fail_on_no_genomes, return an Err if no genomes were detected.
pub fn parse_list_of_genome_fasta_files(m: &clap::ArgMatches, fail_on_no_genomes: bool)
    -> std::result::Result<Vec<String>, String> {

    match m.is_present("genome-fasta-files") {
        true => {
            return Ok(m.values_of("genome-fasta-files").unwrap().map(|s| s.to_string()).collect())
        },
        false => {
            if m.is_present("genome-fasta-directory") {
                let dir = m.value_of("genome-fasta-directory").unwrap();
                let paths = std::fs::read_dir(dir).unwrap();
                let mut genome_fasta_files: Vec<String> = vec!();
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
                                    extension);
                            }
                        },
                        None => {
                            info!("Not using directory entry '{}' as a genome FASTA file",
                                  file.to_str().expect("UTF8 error in filename"));
                        }
                    }
                }
                if genome_fasta_files.len() == 0 {
                    return match fail_on_no_genomes {
                        true => std::result::Result::Err(
                            "Found 0 genomes from the genome-fasta-directory, cannot continue.".to_string()),
                        false => Ok(vec![])
                    }
                }
                return Ok(genome_fasta_files)
            } else {
                return std::result::Result::Err("No genomes options specified".to_string());
            }
        }
    }
}

/// Add --genome-fasta-files and --genome-fasta-directory etc. to a clap App /
/// subcommand. These arguments can later be parsed with
/// parse_list_of_genome_fasta_files().
pub fn add_genome_specification_arguments<'a>(subcommand: clap::App<'a,'a>)
-> clap::App<'a,'a> {
    subcommand
        .arg(Arg::with_name("genome-fasta-files")
            .short("f")
            .long("genome-fasta-files")
            .help("List of fasta files for processing")
            .multiple(true)
            .conflicts_with("genome-fasta-directory")
            .takes_value(true))
        .arg(Arg::with_name("genome-fasta-directory")
            .long("genome-fasta-directory")
            .help("Directory containing fasta files for processing")
            .conflicts_with("genome-fasta-files")
            .takes_value(true))
        .arg(Arg::with_name("genome-fasta-extension")
            .short("x")
            .help("File extension of FASTA files in --genome-fasta-directory")
            .long("genome-fasta-extension")
            // Unsure why, but uncommenting causes test failure (in
            // coverm genome mode where this code was pasted from,
            // not sure about here) - clap bug?
            //.requires("genome-fasta-directory")
            .default_value("fna")
            .takes_value(true))
}
