use clap::Parser;

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

    for file in files {
        println!("* {}", file)
    }
}
