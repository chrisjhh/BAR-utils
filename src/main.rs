use biblearchive::BARFile;
use biblearchive_utils::{details, verse};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
struct Args {
    /// Specify the path to the BARFile to use
    #[arg(short, long)]
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List details about the BARFile
    Details {
        /// Whether to show additional compression details
        #[arg(short, long)]
        compression: bool,
    },
    /// Fetch one or more verses
    Verse {
        /// Reference to the verses to fetch e.g. "Ge 1:1"
        verses: Vec<String>,
    },
    /// Perform a search for matching verses
    Search {
        /// The phrase or pattern to match. eg. "edge of the sword", /prais(es?|ing|ed)/
        #[arg(short, long)]
        matching: Vec<String>,
        /// Phrases or patterns to exclude. eg. "edge of the sword", /prais(es?|ing|ed)/
        #[arg(short, long)]
        notmatching: Vec<String>,
        /// The word(s) that must be present
        #[arg(short, long)]
        word: Vec<String>,
        /// The word(s) that must not be present
        #[arg(short, long)]
        badword: Vec<String>,
        /// The books or chapters to include. eg. NT, OT, Ge, 1Sa..2Ch, "Ps 119"
        #[arg(short, long)]
        include: Vec<String>,
        /// The books or chapters to exclude. eg. NT, OT, Ge, 1Sa..2Ch, "Ps 119"
        #[arg(short = 'x', long)]
        exclude: Vec<String>,
    },
}

fn main() {
    let args = Args::parse();
    let path = args.file.as_deref();
    if path.is_none() {
        println!("Path to BARFile not specified.");
        exit(1);
    }
    let path = path.unwrap();
    let bar = BARFile::open(path);
    if let Err(error) = bar {
        println!("Error opening BARFile.");
        println!("{}", error);
        exit(1);
    }
    let bar = bar.unwrap();

    match &args.command {
        Some(Command::Details { compression }) => {
            details(bar, *compression);
        }
        Some(Command::Verse { verses }) => {
            verse(bar, verses);
        }
        Some(Command::Search {
            matching: _,
            notmatching: _,
            word: _,
            badword: _,
            include: _,
            exclude: _,
        }) => {}
        None => (),
    }
}
