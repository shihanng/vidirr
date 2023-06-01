use clap::Parser;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

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

    let mut file_list = NamedTempFile::new().expect("cannot create temp file"); // TODO: Handle error
    for file in files {
        writeln!(file_list, "{}", file).expect("cannot write")
    }

    println!("{:?}", file_list.path()); // TODO: Remove this.

    Command::new("vi")
        .arg(file_list.path().to_string_lossy().to_string())
        .status()
        .expect("Failed to execute command"); // TODO: Handle error
}
