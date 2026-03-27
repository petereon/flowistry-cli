//! IDE focus mode: for every analyzable place in a function body, compute the
//! bidirectional slice and return it grouped by place span.
//!
//! This mirrors the analysis in `flowistry_ide::focus` but is implemented
//! directly on top of the `flowistry` core crate.

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
    output::{FocusEntry, FocusOutput, LineCol, OwnedSpan},
};

// ── callbacks ─────────────────────────────────────────────────────────────────

struct Callbacks {
    range: ParsedRange,
    output: Option<Result<FocusOutput>>,
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
        self.output = Some(compute(tcx, &self.range));
        rustc_driver::Compilation::Stop
    }
}

// ── core analysis ─────────────────────────────────────────────────────────────

fn compute(tcx: TyCtxt<'_>, range: &ParsedRange) -> Result<FocusOutput> {
    let source_map = tcx.sess.source_map();

    let char_range = CharRange {
        start: CharPos { line: range.start_line, column: range.start_col },
        end: CharPos { line: range.end_line, column: range.end_col },
        filename: Filename::intern(&range.file),
    };

    let cursor_span = FunctionIdentifier::Range(char_range)
        .to_span(tcx)
        .context("could not map location to a source span")?;

    let body_id: BodyId = find_enclosing_bodies(tcx, cursor_span)
        .next()
        .context("no function body found at the given location")?;

    let def_id = tcx.hir_body_owner_def_id(body_id);
    let body_with_facts = get_body_with_borrowck_facts(tcx, def_id);
    let body = &body_with_facts.body;

    let spanner = Spanner::new(tcx, body_id, body);

    // Build one target group per unique MIR span: all (place, location) pairs
    // that share the same source span are analysed together.
    // `ms.span` is `SpanData`; use `.span()` to convert to `Span` for equality.
    let mut span_groups: Vec<(rustc_span::Span, Vec<(Place<'_>, LocationOrArg)>)> = Vec::new();

    for ms in spanner.mir_span_tree.iter() {
        let span = ms.span.span();
        let pairs: Vec<(Place<'_>, LocationOrArg)> =
            ms.locations.iter().map(|&loc| (ms.place, loc)).collect();

        if let Some(entry) = span_groups.iter_mut().find(|(s, _)| *s == span) {
            entry.1.extend(pairs);
        } else {
            span_groups.push((span, pairs));
        }
    }

    // Compute bidirectional slices for every group.
    let targets: Vec<Vec<(Place<'_>, LocationOrArg)>> =
        span_groups.iter().map(|(_, t)| t.clone()).collect();

    let results = infoflow::compute_flow(tcx, body_id, body_with_facts);

    let all_slices =
        infoflow::compute_dependency_spans(&results, targets, Direction::Both, &spanner);

    let entries: Vec<FocusEntry> = span_groups
        .iter()
        .zip(all_slices)
        .filter_map(|((span, _), slice_spans)| {
            let range = span_to_owned(*span, source_map)?;
            let slice: Vec<OwnedSpan> = slice_spans
                .into_iter()
                .filter_map(|s| span_to_owned(s, source_map))
                .collect();
            Some(FocusEntry { range, slice })
        })
        .collect();

    Ok(FocusOutput { entries })
}

// ── public entry point ────────────────────────────────────────────────────────

pub fn run(compiler_args: &[String], range: ParsedRange) -> Result<FocusOutput> {
    let mut cb = Callbacks { range, output: None };
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
