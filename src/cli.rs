use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

/// Information-flow analysis for Rust programs.
///
/// Run from your project root (requires a Cargo.toml in scope):
///
///   cargo flowistry slice src/main.rs:42:7-44:15
///   cargo flowistry spans src/main.rs
///   cargo flowistry influence src/main.rs:10:5
///
/// Positions use 1-based line and column numbers.
#[derive(Parser, Serialize, Deserialize)]
#[clap(name = "flowistry")]
pub struct CliArgs {
    /// Output plain JSON instead of human-readable text
    #[clap(long, global = true)]
    pub json: bool,

    #[clap(subcommand)]
    pub command: Command,
}

impl CliArgs {
    pub fn format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Subcommand, Serialize, Deserialize)]
pub enum Command {
    /// Backward slice: which locations influence the target range
    ///
    /// Examples:
    ///   cargo flowistry slice src/main.rs:42:7
    ///   cargo flowistry slice src/main.rs:42:7-44:15 --json
    #[clap(name = "slice")]
    Slice {
        /// Target location in the form file.rs:L:C or file.rs:L:C-L:C
        location: String,
    },

    /// Forward slice: which locations are influenced by the target range
    ///
    /// Examples:
    ///   cargo flowistry influence src/main.rs:10:5
    ///   cargo flowistry influence src/main.rs:10:5-10:20 --json
    #[clap(name = "influence")]
    Influence {
        /// Target location in the form file.rs:L:C or file.rs:L:C-L:C
        location: String,
    },

    /// List every function body that Flowistry can analyse in a file
    ///
    /// Examples:
    ///   cargo flowistry spans src/main.rs
    ///   cargo flowistry spans src/lib.rs --json | jq '.spans | length'
    #[clap(name = "spans")]
    Spans {
        /// Path to the Rust source file
        file: String,
    },

    /// Show which code regions are relevant to a cursor position (IDE focus mode)
    ///
    /// Returns the backward+forward slice for every analyzable location in the
    /// enclosing function, grouped by place. Useful for building "focus" features
    /// in editors that want to fade out irrelevant code.
    ///
    /// Examples:
    ///   cargo flowistry focus src/main.rs:42:7 --json
    #[clap(name = "focus")]
    Focus {
        /// Cursor position in the form file.rs:L:C or file.rs:L:C-L:C
        location: String,
    },

    /// Print the rustc version this binary was built against and exit
    #[clap(name = "version")]
    Version,
}

impl Command {
    /// Return the source file path embedded in this command, if any.
    pub fn target_file(&self) -> Option<&str> {
        match self {
            Command::Slice { location }
            | Command::Influence { location }
            | Command::Focus { location } => {
                // The file is everything before the first colon that is
                // followed by a digit, i.e. we strip the :L:C[-L:C] suffix.
                // This works for Unix paths; Windows paths with drive letters
                // (C:\...) are not yet supported.
                location
                    .rfind(':')
                    .and_then(|last_colon| {
                        let before_last = &location[..last_colon];
                        before_last
                            .rfind(':')
                            .map(|second_last| &location[..second_last])
                    })
                    .or(Some(location.as_str()))
            }
            Command::Spans { file } => Some(file.as_str()),
            Command::Version => None,
        }
    }
}
