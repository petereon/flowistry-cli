use std::process;

use serde::Serialize;
use serde_json::json;

use crate::cli::OutputFormat;

// ── owned span ────────────────────────────────────────────────────────────────

/// A source range that owns its filename as a plain `String`.
///
/// `CharRange` from `rustc_utils` stores the filename as an opaque `FilenameIndex`
/// (no `Display`). We convert to this type inside the analysis callbacks while we
/// still have access to the `SourceMap`, so the renderers never need rustc types.
#[derive(Clone, Serialize)]
pub struct OwnedSpan {
    pub file: String,
    pub start: LineCol,
    pub end: LineCol,
    pub content: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct LineCol {
    pub line: usize,
    pub col: usize,
}

/// Format an `OwnedSpan` as the canonical `file:L:C-L:C` token string.
pub fn span_to_str(r: &OwnedSpan) -> String {
    format!(
        "{}:{}:{}-{}:{}",
        r.file, r.start.line, r.start.col, r.end.line, r.end.col,
    )
}

fn print_json(value: &impl Serialize) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}

// ── slice / influence ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SliceOutput {
    pub target: OwnedSpan,
    pub dependencies: Vec<OwnedSpan>,
}

pub fn render_slice(
    result: anyhow::Result<SliceOutput>,
    fmt: OutputFormat,
) {
    match result {
        Ok(out) => match fmt {
            OutputFormat::Text => {
                let n = out.dependencies.len();
                println!("Slice of {}", span_to_str(&out.target));
                if let Some(c) = &out.target.content {
                    println!("  {c}");
                }
                println!("{n} dependenc{}:", if n == 1 { "y" } else { "ies" });
                println!();
                for dep in &out.dependencies {
                    println!("  {}", span_to_str(dep));
                    if let Some(c) = &dep.content {
                        println!("    {c}");
                    }
                }
            }
            OutputFormat::Json => print_json(&json!({
                "command": "slice",
                "target": out.target,
                "dependencies": out.dependencies,
            })),
        },
        Err(e) => render_error(&e.to_string(), fmt, 1),
    }
}

// ── spans ─────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SpansOutput {
    pub spans: Vec<OwnedSpan>,
}

pub fn render_spans(result: anyhow::Result<SpansOutput>, file: &str, fmt: OutputFormat) {
    match result {
        Ok(out) => match fmt {
            OutputFormat::Text => {
                let n = out.spans.len();
                println!("Analysable spans in {file}");
                println!("{n} span{}:", if n == 1 { "" } else { "s" });
                println!();
                for span in &out.spans {
                    println!("  {}", span_to_str(span));
                    if let Some(c) = &span.content {
                        println!("    {c}");
                    }
                }
            }
            OutputFormat::Json => print_json(&json!({
                "command": "spans",
                "file": file,
                "spans": out.spans,
            })),
        },
        Err(e) => render_error(&e.to_string(), fmt, 1),
    }
}

// ── focus ─────────────────────────────────────────────────────────────────────

/// A single place-level focus entry.
#[derive(Serialize)]
pub struct FocusEntry {
    pub range: OwnedSpan,
    pub slice: Vec<OwnedSpan>,
}

#[derive(Serialize)]
pub struct FocusOutput {
    pub entries: Vec<FocusEntry>,
}

pub fn render_focus(result: anyhow::Result<FocusOutput>, location: &str, fmt: OutputFormat) {
    match result {
        Ok(out) => match fmt {
            OutputFormat::Text => {
                let n = out.entries.len();
                println!("Focus at {location}");
                println!("{n} place{}:", if n == 1 { "" } else { "s" });
                for entry in &out.entries {
                    println!();
                    println!("  place  {}", span_to_str(&entry.range));
                    if let Some(c) = &entry.range.content {
                        println!("    {c}");
                    }
                    for dep in &entry.slice {
                        println!("  slice  {}", span_to_str(dep));
                        if let Some(c) = &dep.content {
                            println!("    {c}");
                        }
                    }
                }
            }
            OutputFormat::Json => print_json(&json!({
                "command": "focus",
                "location": location,
                "entries": out.entries,
            })),
        },
        Err(e) => render_error(&e.to_string(), fmt, 1),
    }
}

// ── errors ────────────────────────────────────────────────────────────────────

/// Print an error to stderr in the requested format and exit with `code`.
pub fn render_error(message: &str, fmt: OutputFormat, code: i32) {
    match fmt {
        OutputFormat::Text => {
            eprintln!("Error: {message}");
        }
        OutputFormat::Json => {
            let v = json!({ "error": message });
            eprintln!("{}", serde_json::to_string_pretty(&v).unwrap());
        }
    }
    process::exit(code);
}
