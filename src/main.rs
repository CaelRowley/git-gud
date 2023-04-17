use ferris_says::say;
use std::io::{stdout, BufWriter};

struct Cli {
    command: String,
    argument: String,
}

fn main() {
    let command = std::env::args().nth(1).expect("no command given");
    let argument = std::env::args().nth(2).expect("no argument given");
    
    let stdout = stdout();
    let message = command.as_str().to_owned() + " " + argument.as_str();
    let width = message.chars().count();

    let mut writer = BufWriter::new(stdout.lock());
    say(&message, width, &mut writer).unwrap();
}