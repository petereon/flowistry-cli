//! List all analyzable function-body spans in a file.

use anyhow::Result;
use rustc_middle::ty::TyCtxt;
use rustc_span::source_map::SourceMap;
use rustc_utils::source_map::{filename::Filename, find_bodies::find_bodies, range::CharRange};

use crate::{
    driver::run_with_callbacks,
    output::{LineCol, OwnedSpan, SpansOutput},
};

// ── callbacks ─────────────────────────────────────────────────────────────────

struct Callbacks {
    filename: String,
    output: Option<Result<SpansOutput>>,
}

impl rustc_driver::Callbacks for Callbacks {
    // No borrowck setup needed — we only inspect the HIR, not MIR facts.

    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> rustc_driver::Compilation {
        self.output = Some(compute(tcx, &self.filename));
        rustc_driver::Compilation::Stop
    }
}

// ── core analysis ─────────────────────────────────────────────────────────────

fn compute(tcx: TyCtxt<'_>, filename: &str) -> Result<SpansOutput> {
    let source_map = tcx.sess.source_map();

    let source_file = Filename::intern(filename)
        .find_source_file(source_map)
        .map_err(|_| anyhow::anyhow!("file not found in the compiled crate: {filename}\nhint: make sure the path matches what Cargo uses"))?;

    let spans: Vec<OwnedSpan> = find_bodies(tcx)
        .into_iter()
        .map(|(span, _body_id)| span)
        .filter(|span| source_map.lookup_source_file(span.lo()).name == source_file.name)
        .filter_map(|span| span_to_owned(span, source_map))
        .collect();

    Ok(SpansOutput { spans })
}

// ── public entry point ────────────────────────────────────────────────────────

pub fn run(compiler_args: &[String], filename: String) -> Result<SpansOutput> {
    let mut cb = Callbacks { filename, output: None };
    run_with_callbacks(compiler_args, &mut cb)?;
    cb.output.expect("after_analysis was not called")
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn span_to_owned(span: rustc_span::Span, source_map: &SourceMap) -> Option<OwnedSpan> {
    let cr = CharRange::from_span(span, source_map).ok()?;
    let file = cr.filename
        .find_source_file(source_map)
        .ok()
        .map(|sf| sf.name.prefer_local().to_string())
        .unwrap_or_default();
    let content = source_map.span_to_snippet(span).ok();
    Some(OwnedSpan {
        file,
        start: LineCol { line: cr.start.line + 1, col: cr.start.column + 1 },
        end: LineCol { line: cr.end.line + 1, col: cr.end.column + 1 },
        content,
    })
}
