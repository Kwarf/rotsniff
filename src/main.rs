use std::{collections::HashMap, path::PathBuf};

use clap::{Parser, Subcommand};
use database::Database;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use regex::Regex;
use walkdir::WalkDir;

mod database;
mod hash;

#[derive(Parser)]
#[command(about, version)]
struct Args {
    /// Path to the database file
    #[arg(long = "db", value_name = "FILE", default_value_os_t = PathBuf::from(r"./rotsniff.db"))]
    database: PathBuf,

    /// Make `command` more verbose. Actual behavior depends on the command
    #[arg(short, long)]
    verbose: bool,

    /// Restrict commands to files which match regex
    #[arg(short, long)]
    fnfilter: Option<Regex>,

    /// Negate the fnfilter regex match
    #[arg(short = 'F', long)]
    negate_fnfilter: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add files not found in the database
    Append { directory: PathBuf },
    /// Remove entries from the database that no longer exists
    Remove,
    /// Update entries in the database for files that have changed
    Update,
    /// Verify that all files in the database are intact, and that all files have entries in the database
    Verify { directory: PathBuf },
}

fn paths<'a>(args: &'a Args, path: &PathBuf) -> impl Iterator<Item = PathBuf> + 'a {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|x| x.map(|x| x.path().to_owned()).ok())
        .filter(|x| x.is_file())
        .filter(|x| match &args.fnfilter {
            Some(regex) if args.negate_fnfilter => x.to_str().map_or(false, |x| !regex.is_match(x)),
            Some(regex) => x.to_str().map_or(false, |x| regex.is_match(x)),
            None => true,
        })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match &args.command {
        Command::Append { directory } => {
            let mut database = Database::open(&args.database)?;

            let new_hashes = paths(&args, &directory)
                .filter(|x| !database.get(x).is_some())
                .par_bridge()
                .map(|x| {
                    let hash = hash::blake2s(&x).unwrap();
                    if args.verbose {
                        println!("{}: {}", x.as_path().display(), hash);
                    }
                    (x, hash)
                })
                .collect::<HashMap<PathBuf, hash::Hash>>();

            database.extend(new_hashes);
            database.save(&args.database)?;
        }

        Command::Remove => {
            let mut database = Database::open(&args.database)?;

            database.retain(|path| {
                if path.exists() {
                    true
                } else {
                    println!("REMOVED: {}", path.as_path().display());
                    false
                }
            });

            database.save(&args.database)?;
        }

        Command::Update => {
            let mut database = Database::open(&args.database)?;
           
            let updated_hashes = database
                .iter()
                .par_bridge()
                .filter_map(|(path, old)| match hash::blake2s(path) {
                    Ok(new) => {
                        if *old == new {
                            None
                        } else {
                            println!("UPDATED: {}", path.as_path().display());
                            Some((path.clone(), new))
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        None
                    }
                    Err(e) => panic!("{}", e),
                })
                .collect::<Vec<(PathBuf, hash::Hash)>>();

            database.extend(updated_hashes);
            database.save(&args.database)?;
        }

        Command::Verify { directory } => {
            let database = Database::open(&args.database)?;

            let diff = database
                .iter()
                .par_bridge()
                .filter_map(|(path, old)| match hash::blake2s(path) {
                    Ok(new) => {
                        if *old == new {
                            if args.verbose {
                                println!("MATCH: {}", path.as_path().display());
                            }
                            None
                        } else {
                            println!("MODIFIED: {}", path.as_path().display());
                            Some((path.clone(), (Some((old.clone(), new)))))
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        println!("FILE NOT FOUND: {}", path.as_path().display());
                        Some((path.clone(), None))
                    }
                    Err(e) => panic!("{}", e),
                })
                .chain(paths(&args, &directory).par_bridge().filter_map(
                    |x| match database.get(&x) {
                        Some(_) => None,
                        None => {
                            println!("NOT FOUND IN DB: {}", x.as_path().display());
                            Some((x, None))
                        }
                    },
                ))
                .collect::<Vec<(PathBuf, Option<(hash::Hash, hash::Hash)>)>>();

            if diff.len() > 0 {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
