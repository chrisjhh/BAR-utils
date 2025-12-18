use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
pub struct Args {
    /// Specify the path to the BARFile to use
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
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
    Search(SearchArgs),
}

#[derive(Parser)]
pub struct SearchArgs {
    /// The phrase or pattern to match. eg. "edge of the sword", /prais(es?|ing|ed)/
    #[arg(short, long, num_args=1..)]
    matching: Vec<String>,
    /// The word(s) that must be present
    #[arg(short, long, num_args=1..)]
    word: Vec<String>,
    /// The books or chapters to include. eg. NT, OT, Ge, 1Sa..2Ch, "Ps 119"
    #[arg(short, long, num_args=1..)]
    include: Vec<String>,
    /// Count the verses that match in each chapter rather than displaying them all
    #[arg(short, long)]
    count: bool,
    /// The threshold to use when reporting the chapter count
    #[arg(short, long)]
    threshold: Option<u32>,
}

#[macro_export]
macro_rules! oprintln {
    ($out:ident, $($arg:tt)*) => {
        $out.push(format!($($arg)*));
        println!($($arg)*);
    };
}

mod details;
pub use details::details;

mod verse;
pub use verse::verse;

mod search;
pub use search::search;
