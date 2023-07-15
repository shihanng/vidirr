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
}
