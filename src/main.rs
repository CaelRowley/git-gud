// use ferris_says::say;
// use std::io::{stdout, BufWriter};
use clap::Parser;
use git2::{Repository, StatusOptions};

#[derive(Parser)]
struct Args {
    command: String,
    argument: String,
}

fn main() {
    let args = Args::parse();
    if args.command == "clone" {
        let url = args.argument;
        let _repo = match Repository::clone(&url, "./test") {
            Ok(repo) => repo,
            Err(e) => panic!("failed to clone {}", e),
        };
    }
    if args.command == "status" {
        let repo = match Repository::open(".") {
            Ok(repo) => repo,
            Err(e) => panic!("no repo in current dir {}", e),
        };
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        let statuses = match repo.statuses(Some(&mut opts)) {
            Ok(statuses) => statuses,
            Err(e) => panic!("oops {}", e),
        };
        for entry in statuses.iter() {
            println!("{}", entry.path().unwrap());
        }
    }
    // let stdout = stdout();
    // let message = command.clone() + " " + &argument;
    // let width = message.chars().count();

    // let mut writer = BufWriter::new(stdout.lock());
    // say(&message, width, &mut writer).unwrap();
} 