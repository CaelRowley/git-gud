use std::process::Command;
use colored::*;


pub fn default(args: Vec<String>) {
    let default_command = "git";

    let command_str = format!("{} {}", default_command, args.join(" "));
    println!("Running default command: {}", command_str.bold());

    let output = Command::new(default_command)
        .args(args)
        .output()
        .expect(&format!("Failed to execute command '{}'", default_command));

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        if !result.is_empty() {
            println!("Output: {}", result.bold());
        }
    } else {
        let result = String::from_utf8_lossy(&output.stderr);
        if !result.is_empty() {
            println!("{}", format!("Error: {}", result.bold()).red());
        }
    }
}