use std::fs;
use std::io::{self, BufRead, ErrorKind};

#[derive(PartialEq, Debug)]
pub struct Parsed {
    pub files: Vec<String>,
    pub dirs: Vec<String>,
}

// https://stackoverflow.com/questions/38183551/concisely-initializing-a-vector-of-strings
pub fn parse_args<F>(args: &[String], read_from: F) -> io::Result<Parsed>
where
    F: Fn() -> Box<dyn BufRead>,
{
    let mut parsed = Parsed {
        files: Vec::new(),
        dirs: Vec::new(),
    };

    for arg in args {
        if arg == "-" {
            let stdin = read_from();
            for line in stdin.lines() {
                match line {
                    Ok(line) => parsed.files.push(line),
                    Err(err) => return Err(err),
                }
            }
            continue;
        }

        match fs::metadata(arg) {
            Ok(metadata) => {
                if metadata.is_dir() {
                    let entries = fs::read_dir(arg)?;
                    for entry in entries {
                        let entry = entry?;

                        match entry.path().to_str() {
                            Some(path) => {
                                if entry.path().is_dir() {
                                    parsed.dirs.push(path.to_string())
                                } else {
                                    parsed.files.push(path.to_string())
                                }
                            }
                            None => {} // TODO: Log here
                        }
                    }
                } else {
                    parsed.files.push(arg.to_string())
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    parsed.files.push(arg.to_string())
                } else {
                    return Err(e);
                }
            }
        }
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    #[test]
    fn test_parse_args() {
        let input = vec!["abc".to_string(), "xyz".to_string()];
        let expected = Parsed {
            files: input.clone(),
            dirs: Vec::new(),
        };
        let result = parse_args(&input, || {
            Box::new(BufReader::new(Cursor::new(String::new())))
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    // If pass "-", we should get the values from reader.
    #[test]
    fn test_parse_args_read_from() {
        let input = vec!["-".to_string(), "---".to_string(), "-".to_string()];
        let expected = Parsed {
            files: vec![
                "./src/testdata".to_string(),
                "abc".to_string(),
                "xyz".to_string(),
                "---".to_string(),
                "./src/testdata".to_string(),
                "abc".to_string(),
                "xyz".to_string(),
            ],
            dirs: Vec::new(),
        };
        let result = parse_args(&input, || {
            let read_from_string = "./src/testdata\nabc\nxyz".to_owned();
            Box::new(BufReader::new(Cursor::new(read_from_string)))
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_parse_args_dir() {
        let input = vec!["./src/testdata".to_string(), "xyz".to_string()];
        let expected = Parsed {
            files: vec![
                "./src/testdata/file2".to_string(),
                "./src/testdata/file1".to_string(),
                "xyz".to_string(),
            ],
            dirs: vec!["./src/testdata/dir1".to_string()],
        };
        let result = parse_args(&input, || {
            Box::new(BufReader::new(Cursor::new(String::new())))
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }
}
