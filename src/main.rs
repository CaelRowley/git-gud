use std::env;

mod clone;
mod status;
mod push;
mod default;


fn main() {
    let cli_args: Vec<String> = env::args().collect();

    let command = cli_args[1].clone();

    let mut args = Vec::new();
    for arg in cli_args.iter().skip(1) {
        args.push(arg.clone());
    }

    match command.as_str() {
        "clone" | "c" => clone::clone(args),
        "status" | "s" => status::status(args),
        "push" | "p" => push::push(args),
        _ => default::default(args),
    }
}