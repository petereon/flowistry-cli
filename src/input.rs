use anyhow::{Context, Result};

/// A source range with 1-based line and column numbers.
#[derive(Clone, Debug)]
pub struct ParsedRange {
    pub file: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

/// Parse a location string of the form:
///
///   file.rs:L:C         — point (treated as a zero-width range)
///   file.rs:L:C-L:C     — explicit start–end range
///
/// Lines and columns are 1-based. File paths must not contain bare colons
/// (Windows drive-letter paths like `C:\...` are not yet supported).
///
/// # Errors
///
/// Returns an error with a human-readable message if the format is not recognised
/// or if any numeric component cannot be parsed.
pub fn parse_range(s: &str) -> Result<ParsedRange> {
    // Split on ':'. For "src/lib.rs:42:7-44:15" we get:
    //   ["src/lib.rs", "42", "7-44", "15"]
    // For "src/lib.rs:42:7" we get:
    //   ["src/lib.rs", "42", "7"]
    let parts: Vec<&str> = s.split(':').collect();

    match parts.as_slice() {
        // --- point: file:L:C ---
        [file, line, col] => {
            let line = parse_num(line, "line")?;
            let col = parse_num(col, "column")?;
            Ok(ParsedRange {
                file: file.to_string(),
                start_line: line,
                start_col: col,
                end_line: line,
                end_col: col,
            })
        }

        // --- range: file:SL:SC-EL:EC ---
        // After splitting on ':' we get ["file", "SL", "SC-EL", "EC"]
        [file, start_line, sc_dash_el, end_col] => {
            let (sc_str, el_str) = sc_dash_el
                .split_once('-')
                .with_context(|| format!(
                    "expected start_col-end_line in {:?}, e.g. '7-44'",
                    sc_dash_el
                ))?;
            let start_line = parse_num(start_line, "start line")?;
            let start_col = parse_num(sc_str, "start column")?;
            let end_line = parse_num(el_str, "end line")?;
            let end_col = parse_num(end_col, "end column")?;
            Ok(ParsedRange {
                file: file.to_string(),
                start_line,
                start_col,
                end_line,
                end_col,
            })
        }

        _ => anyhow::bail!(
            "invalid location {:?}\n\
             expected  file.rs:L:C  or  file.rs:L:C-L:C\n\
             example   src/main.rs:42:7  or  src/main.rs:42:7-44:15",
            s
        ),
    }
}

fn parse_num(s: &str, label: &str) -> Result<usize> {
    s.parse::<usize>()
        .with_context(|| format!("{label} must be a number, got {:?}", s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point() {
        let r = parse_range("src/main.rs:42:7").unwrap();
        assert_eq!(r.file, "src/main.rs");
        assert_eq!(r.start_line, 42);
        assert_eq!(r.start_col, 7);
        assert_eq!(r.end_line, 42);
        assert_eq!(r.end_col, 7);
    }

    #[test]
    fn range() {
        let r = parse_range("src/main.rs:42:7-44:15").unwrap();
        assert_eq!(r.file, "src/main.rs");
        assert_eq!(r.start_line, 42);
        assert_eq!(r.start_col, 7);
        assert_eq!(r.end_line, 44);
        assert_eq!(r.end_col, 15);
    }

    #[test]
    fn bad_format() {
        assert!(parse_range("src/main.rs").is_err());
        assert!(parse_range("src/main.rs:abc:7").is_err());
    }
}
