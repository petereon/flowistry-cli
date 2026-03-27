//! Backward and forward slicing.
//!
//! `run(compiler_args, range, direction)` drives rustc, locates MIR locations
//! whose source span overlaps the requested range, then calls
//! `flowistry::infoflow::compute_dependency_spans` to obtain the result.

use anyhow::{Context, Result};
use flowistry::infoflow::{self, Direction};
use rustc_hir::BodyId;
use rustc_middle::{mir::Place, ty::TyCtxt};
use rustc_span::source_map::SourceMap;
use rustc_utils::{
    mir::{
        borrowck_facts::get_body_with_borrowck_facts,
        location_or_arg::LocationOrArg,
    },
    source_map::{
        filename::Filename,
        find_bodies::find_enclosing_bodies,
        range::{CharPos, CharRange, FunctionIdentifier, ToSpan},
        spanner::Spanner,
    },
};

use crate::{
    driver::{configure, run_with_callbacks},
    input::ParsedRange,
    output::{LineCol, OwnedSpan, SliceOutput},
};

// ── callbacks ─────────────────────────────────────────────────────────────────

struct Callbacks {
    range: ParsedRange,
    direction: Direction,
    output: Option<Result<SliceOutput>>,
}

impl rustc_driver::Callbacks for Callbacks {
    fn config(&mut self, config: &mut rustc_interface::Config) {
        configure(config);
    }

    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> rustc_driver::Compilation {
        self.output = Some(compute(tcx, &self.range, self.direction));
        rustc_driver::Compilation::Stop
    }
}

// ── core analysis ─────────────────────────────────────────────────────────────

fn compute<'tcx>(tcx: TyCtxt<'tcx>, range: &ParsedRange, direction: Direction) -> Result<SliceOutput> {
    let char_range = to_char_range(range);
    let source_map = tcx.sess.source_map();

    let target_span = FunctionIdentifier::Range(char_range.clone())
        .to_span(tcx)
        .context("could not map location to a source span — check that the file path and line/column are correct")?;

    let body_id: BodyId = find_enclosing_bodies(tcx, target_span)
        .next()
        .context("no function body found at the given location")?;

    let def_id = tcx.hir_body_owner_def_id(body_id);
    let body_with_facts = get_body_with_borrowck_facts(tcx, def_id);
    let body = &body_with_facts.body;

    let spanner = Spanner::new(tcx, body_id, body);

    // Collect every (place, location) pair whose MIR span overlaps our target.
    // `ms.span` is `SpanData`; convert to `Span` via `.span()` for comparison.
    let targets: Vec<(Place<'tcx>, LocationOrArg)> = spanner
        .mir_span_tree
        .iter()
        .filter(|ms| overlaps(ms.span.span(), target_span))
        .flat_map(|ms| {
            let place = ms.place;
            ms.locations.iter().map(move |&loc| (place, loc))
        })
        .collect();

    anyhow::ensure!(
        !targets.is_empty(),
        "no analyzable locations found at {}\n\
         hint: run `cargo flowistry spans {}` to see what ranges are available",
        range_str(range),
        range.file,
    );

    let results = infoflow::compute_flow(tcx, body_id, body_with_facts);

    let dep_spans = infoflow::compute_dependency_spans(
        &results,
        vec![targets],
        direction,
        &spanner,
    );

    let target_owned = span_to_owned(target_span, source_map)
        .expect("target span must be convertible");
    let dependencies: Vec<OwnedSpan> = dep_spans
        .into_iter()
        .flatten()
        .filter_map(|span| span_to_owned(span, source_map))
        .collect();

    Ok(SliceOutput { target: target_owned, dependencies })
}

// ── public entry point ────────────────────────────────────────────────────────

pub fn run(
    compiler_args: &[String],
    range: ParsedRange,
    direction: Direction,
) -> Result<SliceOutput> {
    let mut cb = Callbacks { range, direction, output: None };
    run_with_callbacks(compiler_args, &mut cb)?;
    cb.output.expect("after_analysis was not called")
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn to_char_range(r: &ParsedRange) -> CharRange {
    CharRange {
        start: CharPos { line: r.start_line, column: r.start_col },
        end: CharPos { line: r.end_line, column: r.end_col },
        filename: Filename::intern(&r.file),
    }
}

fn range_str(r: &ParsedRange) -> String {
    format!("{}:{}:{}-{}:{}", r.file, r.start_line, r.start_col, r.end_line, r.end_col)
}

/// Returns true when span `a` and span `b` overlap in source.
fn overlaps(a: rustc_span::Span, b: rustc_span::Span) -> bool {
    a.lo() <= b.hi() && b.lo() <= a.hi()
}

/// Convert a `rustc_span::Span` to an `OwnedSpan`, including the source snippet.
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
