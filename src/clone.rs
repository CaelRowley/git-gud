use git2::Repository;
use colored::*;


pub fn clone(args: Vec<String>) {
    let url = args[1].to_owned();
    let repo = match Repository::clone(&url, "./test") {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to clone repo: {}", e),
    };
    println!("Repo cloned to: {}\n", repo.path().parent().unwrap().to_string_lossy().bold());
}
