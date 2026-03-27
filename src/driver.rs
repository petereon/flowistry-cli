//! Thin wrapper around `rustc_driver::run_compiler` that adds the flags
//! Flowistry requires for accurate borrow-checker–aware analysis.

use anyhow::Result;
use rustc_utils::mir::borrowck_facts;

/// Run the Rust compiler with `callbacks`, injecting the extra flags that
/// Flowistry needs:
///
/// * `-Z identify-regions` — stable region identifiers for alias analysis
/// * `-Z mir-opt-level=0`  — disable MIR optimisations that would obscure flow
/// * `-A warnings`         — silence warnings so they don't pollute our output
/// * `-Z maximal-hir-to-mir-coverage` — richer span coverage for MIR locations
pub fn run_with_callbacks(
    compiler_args: &[String],
    callbacks: &mut (dyn rustc_driver::Callbacks + Send),
) -> Result<()> {
    let mut args = compiler_args.to_vec();
    args.extend(
        "-Z identify-regions -Z mir-opt-level=0 -A warnings -Z maximal-hir-to-mir-coverage"
            .split_whitespace()
            .map(str::to_owned),
    );
    rustc_driver::catch_fatal_errors(|| rustc_driver::run_compiler(&args, callbacks))
        .map_err(|_| anyhow::anyhow!("rustc compilation failed"))
}

/// Configure a `rustc_interface::Config` for Flowistry's borrow-checker
/// fact collection. Call this from `Callbacks::config`.
pub fn configure(config: &mut rustc_interface::Config) {
    borrowck_facts::enable_mir_simplification();
    config.override_queries = Some(borrowck_facts::override_queries);
}
