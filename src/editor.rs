use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::Write;

pub fn write_with_ids<W: Write>(
    output: &mut W,
    sources: &[String],
) -> std::io::Result<HashMap<usize, String>> {
    let mut items = HashMap::new();
    let padding = (sources.len() + 1).to_string().len();

    for (i, file) in sources.iter().enumerate() {
        items.insert(i + 1, file.to_string());
        writeln!(output, "{:<p$} {}", i + 1, file, p = padding)?
    }
    Ok(items)
}

#[derive(PartialEq, Debug)]
struct ParsedLine {
    num: usize,
    filename: String,
}

fn parse_line(input: &str) -> Result<Option<ParsedLine>> {
    let trimmed = input.trim_start();

    if trimmed.is_empty() {
        return Ok(None);
    }

    match trimmed.chars().position(|c| !c.is_numeric()) {
        Some(0) => Err(anyhow!("no number found")),
        Some(idx) => {
            let remain = trimmed[idx..].chars();
            let mut peeker = remain.peekable();

            // Remove single space after number.
            // Treat the space as separator.
            let filename_idx = match peeker.peek() {
                Some(&' ') => idx + 1,
                _ => idx,
            };

            Ok(Some(ParsedLine {
                num: trimmed[..idx].parse::<usize>()?,
                filename: trimmed[filename_idx..].to_string(),
            }))
        }
        None => Ok(Some(ParsedLine {
            num: trimmed.parse::<usize>()?,
            filename: "".to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_with_ids() {
        let files = vec![
            "./src/testdata/file2".to_string(),
            "./src/testdata/file1".to_string(),
            "xyz".to_string(),
        ];

        let expected: HashMap<usize, String> = HashMap::from([
            (1, "./src/testdata/file2".to_string()),
            (2, "./src/testdata/file1".to_string()),
            (3, "xyz".to_string()),
        ]);

        let mut buffer = Vec::new();
        let result = write_with_ids(&mut buffer, &files);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
        assert_eq!(
            buffer,
            br"1 ./src/testdata/file2
2 ./src/testdata/file1
3 xyz
"
        )
    }

    #[test]
    fn test_parse_line_empty() {
        let input = "";
        let parsed = parse_line(input);
        assert!(parsed.unwrap().is_none())
    }

    #[test]
    fn test_parse_line_skip() {
        let input = "   \t  \n  ";
        let parsed = parse_line(input);
        assert!(parsed.unwrap().is_none())
    }

    #[test]
    fn test_parse_line_123() {
        let input = "123";
        let parsed = parse_line(input);
        assert_eq!(
            parsed.unwrap().unwrap(),
            ParsedLine {
                num: 123,
                filename: "".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_line_123_space() {
        let input = "123 ";
        let parsed = parse_line(input);
        assert_eq!(
            parsed.unwrap().unwrap(),
            ParsedLine {
                num: 123,
                filename: "".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_line_no_number() {
        let input = "     file with space 123 ";
        let parsed = parse_line(input);
        assert_eq!(parsed.unwrap_err().to_string(), "no number found");
    }

    #[test]
    fn test_parse_line() {
        let input = "  345   file with space 123 ";
        let parsed = parse_line(input);
        assert_eq!(
            parsed.unwrap().unwrap(),
            ParsedLine {
                num: 345,
                filename: "  file with space 123 ".to_string(),
            }
        );
    }
}
