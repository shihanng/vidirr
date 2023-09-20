use crate::editor::ParsedLine;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpsError {
    #[error("{0} does not exist")]
    NotFound(String),

    #[error("failed to rename {from:?} to {to:?}: {source:?}")]
    FailRename {
        #[source]
        source: std::io::Error,
        from: String,
        to: String,
    },

    #[error("failed to copy {from:?} to {to:?}: {source:?}")]
    FailCopy {
        #[source]
        source: std::io::Error,
        from: String,
        to: String,
    },
}

pub trait Operation {
    fn rename(&self, from: &str, to: &str) -> Result<()> {
        if let Err(source) = fs::rename(from, to) {
            bail!(OpsError::FailRename {
                source,
                from: from.to_string(),
                to: to.to_string()
            })
        }
        Ok(())
    }

    fn copy(&self, from: &str, to: &str) -> Result<()> {
        if let Err(source) = fs::copy(from, to) {
            bail!(OpsError::FailCopy {
                source,
                from: from.to_string(),
                to: to.to_string()
            })
        }
        Ok(())
    }
}

pub struct FS;

impl Operation for FS {}

pub struct Operator {
    items: HashMap<usize, String>,
    dones: HashMap<usize, String>,
}

impl Operator {
    // new takes items as arguments
    pub fn new(items: HashMap<usize, String>) -> Self {
        let l = items.len();
        Self {
            items,
            dones: HashMap::with_capacity(l),
        }
    }

    pub fn apply_changes<T: Operation>(&mut self, parsed_line: ParsedLine, ops: T) -> Result<()> {
        let num = &parsed_line.num;
        let new_name = parsed_line.filename;
        let done = self.dones.get(num);
        let item = self.items.get(num);
        let is_copy = done.is_some();

        // Check if number part is in items or dones.
        if item.is_none() && !is_copy {
            bail!("unknown item number {}", parsed_line.num);
        } else if is_copy || *item.unwrap() != new_name {
            // Handle move or copy if filename is different or is_copy is true.

            // If target filename is empty, skip.
            if new_name.is_empty() {
                return Ok(());
            }

            let src = match done {
                Some(name) => name,
                None => item.unwrap(),
            }
            .clone();

            // Check if src exists.
            match Path::new(&src).try_exists() {
                Ok(false) => {
                    self.items.remove(num);
                    bail!(OpsError::NotFound(src))
                }
                Err(e) => bail!(e),
                _ => {}
            }

            let new_name_path = Path::new(&new_name);

            // Deal with swaps.
            if let Ok(true) = new_name_path.try_exists() {
                let tmp_name = get_unique_tmp_name(&new_name);
                ops.rename(&new_name, &tmp_name)?;

                // TODO: log
                // print "'$name' -> '$tmp'\n";

                self.update_items(&new_name, &tmp_name);
            }

            // Make sure directory to new_name exists.
            if let Some(parent) = new_name_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }

            if is_copy {
                ops.copy(&src, &new_name)?;
            } else {
                ops.rename(&src, &new_name)?;
            }

            // If name is directory, update all items that start with name.
            if new_name_path.is_dir() {
                self.update_dir(&src, &new_name);
            }
            // TODO: log
            // if ($opt_verbose) {
            //   print "'$src' => '$name'\n" unless $iscopy;
            //   print "'$src' ~> '$name'\n" if $iscopy;
            // }
        }

        self.dones.insert(*num, new_name);
        self.items.remove(num);

        Ok(())
    }

    fn update_items(&mut self, from: &str, to: &str) {
        for (_, name) in self.items.iter_mut() {
            if name == from {
                *name = to.to_string();
            }
        }
    }

    fn update_dir(&mut self, from: &str, to: &str) {
        for (_, name) in self.items.iter_mut() {
            if name.starts_with(from) {
                *name = to.to_string() + &name[from.len()..];
            }
        }
    }
}

fn get_unique_tmp_name(name: &str) -> String {
    let mut new_name = name.to_string();
    new_name.push('~');

    let mut i = 1;
    while Path::new(&new_name).try_exists().unwrap() {
        new_name.push_str(&i.to_string());
        i += 1;
    }
    new_name
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;

    #[test]
    fn test_apply_changes_unknown_number() {
        let items = HashMap::from([(2, "file_2".to_string())]);

        let mut operator = Operator::new(items.to_owned());

        // Operate on item that does not exist in Operator.items.
        let res = operator.apply_changes(
            ParsedLine {
                num: 1,
                filename: "file_one".to_string(),
            },
            FS,
        );

        assert_eq!(res.unwrap_err().to_string(), "unknown item number 1");
        assert_eq!(operator.items, items);
        assert!(operator.dones.is_empty());
    }

    #[test]
    fn test_apply_changes_empty_filename() {
        let items = HashMap::from([(1, "file_1".to_string())]);

        let mut operator = Operator::new(items.clone());

        let res = operator.apply_changes(
            ParsedLine {
                num: 1,
                filename: "".to_string(),
            },
            FS,
        );

        assert!(res.is_ok());
        assert_eq!(operator.items, items);
        assert!(operator.dones.is_empty());
    }

    #[test]
    fn test_apply_changes_src_not_exists() {
        let items = HashMap::from([(1, "file_1".to_string())]);

        let mut operator = Operator::new(items);

        // File we want to rename from does not exist.
        let res = operator.apply_changes(
            ParsedLine {
                num: 1,
                filename: "file_one".to_string(),
            },
            FS,
        );

        assert_eq!(res.unwrap_err().to_string(), "file_1 does not exist");
        assert!(operator.items.is_empty());
        assert!(operator.dones.is_empty());
    }

    #[test]
    fn test_apply_changes_src_exists() {
        let temp = assert_fs::TempDir::new().unwrap();
        let temp_str = temp.to_str().unwrap();
        let file_1 = temp.child("file_1");
        file_1.touch().unwrap();

        let items = HashMap::from([(1, file_1.path().to_str().unwrap().to_string())]);

        let mut operator = Operator::new(items);

        let want_dones = HashMap::from([(1, temp_str.to_owned() + "/file_one")]);

        let res = operator.apply_changes(
            ParsedLine {
                num: 1,
                filename: temp_str.to_owned() + "/file_one",
            },
            FS,
        );

        assert!(res.is_ok());
        assert!(operator.items.is_empty());
        assert_eq!(operator.dones, want_dones);

        temp.child("file_1").assert(predicate::path::missing());
        temp.child("file_one").assert(predicate::path::exists());
    }

    #[test]
    fn test_apply_changes_swap() {
        let temp = assert_fs::TempDir::new().unwrap();
        let temp_str = temp.to_str().unwrap();
        let file_1 = temp.child("file_1");
        let file_2 = temp.child("file_2");
        file_1.touch().unwrap();
        file_2.touch().unwrap();

        let items = [(1, file_1), (2, file_2)]
            .into_iter()
            .map(|(k, v)| (k, v.to_str().unwrap().to_string()))
            .collect();

        let mut operator = Operator::new(items);

        let want_items = HashMap::from([(2, temp_str.to_owned() + "/file_2~")]);
        let want_dones = HashMap::from([(1, temp_str.to_owned() + "/file_2")]);

        // Rename item 1 to the same name as item 2.
        // Therefore, item 2 has to be renamed to item 2~.
        let res = operator.apply_changes(
            ParsedLine {
                num: 1,
                filename: temp_str.to_owned() + "/file_2",
            },
            FS,
        );

        assert!(res.is_ok());
        assert_eq!(operator.items, want_items);
        assert_eq!(operator.dones, want_dones);

        temp.child("file_1").assert(predicate::path::missing());
        temp.child("file_2").assert(predicate::path::exists());
        temp.child("file_2~").assert(predicate::path::exists());
    }

    #[test]
    fn test_apply_changes_copy() {
        let temp = assert_fs::TempDir::new().unwrap();
        let temp_str = temp.to_str().unwrap();
        let file_1 = temp.child("file_1");
        file_1.touch().unwrap();

        let items = [(1, file_1)]
            .into_iter()
            .map(|(k, v)| (k, v.to_str().unwrap().to_string()))
            .collect();

        let mut operator = Operator::new(items);

        // First call changes nothing because the name is the same as in items.
        {
            let want_dones = HashMap::from([(1, temp_str.to_owned() + "/file_1")]);

            let res = operator.apply_changes(
                ParsedLine {
                    num: 1,
                    filename: temp_str.to_owned() + "/file_1",
                },
                FS,
            );

            assert!(res.is_ok());
            assert!(operator.items.is_empty());
            assert_eq!(operator.dones, want_dones);
        }

        // Second call is a copy because it has the same number.
        {
            let want_dones = HashMap::from([(1, temp_str.to_owned() + "/file_1_copy")]);

            let res = operator.apply_changes(
                ParsedLine {
                    num: 1,
                    filename: temp_str.to_owned() + "/file_1_copy",
                },
                FS,
            );

            assert!(res.is_ok());
            assert!(operator.items.is_empty());
            assert_eq!(operator.dones, want_dones);
        }

        temp.child("file_1").assert(predicate::path::exists());
        temp.child("file_1_copy").assert(predicate::path::exists());
    }

    #[test]
    fn test_apply_changes_rename_directory() {
        let temp = assert_fs::TempDir::new().unwrap();
        let temp_str = temp.to_str().unwrap();
        let temp_sub = temp.child("dir_1");
        let temp_sub_str = temp_sub.to_str().unwrap();
        temp_sub.create_dir_all().unwrap();
        let file_1 = temp_sub.child("file_1");
        file_1.touch().unwrap();

        let items = HashMap::from([
            (1, temp_sub_str.to_owned()),
            (2, file_1.path().to_str().unwrap().to_string()),
        ]);

        let mut operator = Operator::new(items);

        let want_items = HashMap::from([(2, temp_str.to_owned() + "/dir_one/file_1")]);
        let want_dones = HashMap::from([(1, temp_str.to_owned() + "/dir_one")]);

        let res = operator.apply_changes(
            ParsedLine {
                num: 1,
                filename: temp_str.to_owned() + "/dir_one",
            },
            FS,
        );

        assert!(res.is_ok());
        assert_eq!(operator.items, want_items);
        assert_eq!(operator.dones, want_dones);

        temp.child("dir_1").assert(predicate::path::missing());
        temp.child("dir_one/file_1")
            .assert(predicate::path::exists());
    }

    #[test]
    fn test_apply_changes_subdirectory() {
        let temp = assert_fs::TempDir::new().unwrap();
        let temp_str = temp.to_str().unwrap();
        let file_1 = temp.child("file_1");
        file_1.touch().unwrap();

        let items = HashMap::from([(2, file_1.path().to_str().unwrap().to_string())]);

        let mut operator = Operator::new(items);

        let want_dones = HashMap::from([(2, temp_str.to_owned() + "/subdir/file_one")]);

        let res = operator.apply_changes(
            ParsedLine {
                num: 2,
                filename: temp_str.to_owned() + "/subdir/file_one",
            },
            FS,
        );

        assert!(res.is_ok());
        assert!(operator.items.is_empty());
        assert_eq!(operator.dones, want_dones);

        temp.child("subdir/file_one")
            .assert(predicate::path::exists());
    }

    #[test]
    fn test_update_dir() {
        // This is okay because we will catch and ignore using
        // NotFound error.
        let mut operator = Operator::new(
            [(1, "./src/testdata/file2"), (2, "./src/testdata/file1")]
                .into_iter()
                .map(|(k, v)| (k, v.to_string()))
                .collect(),
        );

        let want_items = [(1, "./src/test/file2"), (2, "./src/test/file1")]
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();

        operator.update_dir("./src/testdata/", "./src/test/");

        assert_eq!(operator.items, want_items);
    }

    #[test]
    fn test_update_dir_odd_case() {
        // This is okay because we will catch and ignore using
        // NotFound error.
        let mut operator = Operator::new(
            [(1, "./src/testdata/file2"), (2, "./src/testdata/file1")]
                .into_iter()
                .map(|(k, v)| (k, v.to_string()))
                .collect(),
        );

        let want_items = [(1, "./src/testfile2"), (2, "./src/testfile1")]
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();

        operator.update_dir("./src/testdata/", "./src/test");

        assert_eq!(operator.items, want_items);
    }

    #[test]
    fn test_get_unique_tmp_name_first_try() {
        let temp = assert_fs::TempDir::new().unwrap();
        let temp_str = temp.to_str().unwrap();
        let file_1 = temp.child("file_1");
        file_1.touch().unwrap();

        let got = get_unique_tmp_name(&(temp_str.to_owned() + "/file_1"));
        assert_eq!(temp_str.to_owned() + "/file_1~", got);
    }

    #[test]
    fn test_get_unique_tmp_name_1() {
        let temp = assert_fs::TempDir::new().unwrap();
        let temp_str = temp.to_str().unwrap();
        let file_1a = temp.child("file_1");
        let file_1b = temp.child("file_1~");
        file_1a.touch().unwrap();
        file_1b.touch().unwrap();

        let got = get_unique_tmp_name(&(temp_str.to_owned() + "/file_1"));
        assert_eq!(temp_str.to_owned() + "/file_1~1", got);
    }
}
