#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;

pub mod analysis;
pub mod cli;
pub mod driver;
pub mod input;
pub mod output;

use std::{borrow::Cow, path::PathBuf, process};

use cli::{CliArgs, Command};
use flowistry::infoflow::Direction;
use rustc_interface::interface::Result as RustcResult;
use rustc_plugin::{CrateFilter, RustcPlugin, RustcPluginArgs, Utf8Path};
use clap::Parser;

pub struct FlowistryCliPlugin;

impl RustcPlugin for FlowistryCliPlugin {
    type Args = CliArgs;

    fn driver_name(&self) -> Cow<'static, str> {
        "flowistry-driver".into()
    }

    fn version(&self) -> Cow<'static, str> {
        env!("CARGO_PKG_VERSION").into()
    }

    fn args(&self, _target_dir: &Utf8Path) -> RustcPluginArgs<CliArgs> {
        // When invoked as `cargo flowistry <subcmd>`, argv is:
        //   [cargo-flowistry, flowistry, <subcmd>, ...]
        // skip(1) drops the binary name; clap sees [flowistry, <subcmd>, ...] and treats
        // "flowistry" as the program name, <subcmd> as the subcommand.
        let args = CliArgs::parse_from(std::env::args().skip(1));

        // Handle commands that do not need rustc at all.
        if let Command::Version = &args.command {
            let rustc_ver = rustc_interface::util::rustc_version_str().unwrap_or("unknown");
            println!("flowistry-cli {}", env!("CARGO_PKG_VERSION"));
            println!("rustc {rustc_ver}");
            process::exit(0);
        }

        let file = args.command.target_file()
            .expect("all commands except 'version' require a file path")
            .to_string();

        RustcPluginArgs {
            filter: CrateFilter::CrateContainingFile(PathBuf::from(&file)),
            args,
        }
    }

    fn run(self, compiler_args: Vec<String>, plugin_args: CliArgs) -> RustcResult<()> {
        let fmt = plugin_args.format();
        match plugin_args.command {
            Command::Slice { location } => {
                match input::parse_range(&location) {
                    Ok(range) => {
                        let result = analysis::slice::run(&compiler_args, range, Direction::Backward);
                        output::render_slice(result, fmt);
                    }
                    Err(e) => output::render_error(&e.to_string(), fmt, 3),
                }
            }
            Command::Influence { location } => {
                match input::parse_range(&location) {
                    Ok(range) => {
                        let result = analysis::slice::run(&compiler_args, range, Direction::Forward);
                        output::render_slice(result, fmt);
                    }
                    Err(e) => output::render_error(&e.to_string(), fmt, 3),
                }
            }
            Command::Spans { file } => {
                let result = analysis::spans::run(&compiler_args, file.clone());
                output::render_spans(result, &file, fmt);
            }
            Command::Focus { location } => {
                match input::parse_range(&location) {
                    Ok(range) => {
                        let result = analysis::focus::run(&compiler_args, range);
                        output::render_focus(result, &location, fmt);
                    }
                    Err(e) => output::render_error(&e.to_string(), fmt, 3),
                }
            }
            Command::Version => unreachable!("handled in args()"),
        }
        Ok(())
    }
}
