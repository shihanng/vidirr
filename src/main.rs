use clap::Parser;
use std::fs::File;
use std::io::Write;
use std::io::{self, BufRead};
use std::process::Command;
use tempfile::NamedTempFile;
use vidirr::ops;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    sort: bool,
    #[arg(short, long)]
    verbose: bool,
    files: Vec<String>,
}

fn main() {
    let args = Cli::parse();
    let mut files = args.files;

    if files.is_empty() {
        files.push("./".to_string())
    }

    let target = vidirr::parse_args(&files, || Box::new(io::stdin().lock())).expect("cannot parse"); // TODO: Handle error

    let mut file_list = NamedTempFile::new().expect("cannot create temp file"); // TODO: Handle error

    let items =
        vidirr::editor::write_with_ids(&mut file_list, &target.all()).expect("cannot write");

    println!("{:?}", file_list.path()); // TODO: Remove this.

    Command::new("vi")
        .arg(file_list.path().to_string_lossy().to_string())
        .status()
        .expect("Failed to execute command"); // TODO: Handle error
                                              //
    let reader = io::BufReader::new(File::open(file_list.path()).expect("cannot open file"));

    let mut operator = ops::Operator::new(items);
    for line in reader.lines() {
        let l = line.expect("cannot read line"); // TODO: Handle error

        let parsed_line = vidirr::editor::parse_line(&l).expect("cannot parse line");
        //    die "$0: unable to parse line \"$_\", aborting\n";

        match parsed_line {
            Some(parsed_line) => match operator.apply_changes(parsed_line, ops::FS) {
                Ok(_) => {}
                Err(err) => {
                    if let Some(e) = err.downcast_ref::<ops::OpsError>() {
                        println!("{}", e)
                    } else {
                        panic!("ahhhhhhh")
                    }
                }
            },
            None => continue, // Skip empty line.
        }
    }
    // Remove
}
