use git2::{Cred, Repository, PushOptions, RemoteCallbacks};
use colored::*;


pub fn push(_args: Vec<String>) {
    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(e) => panic!("No repo in current dir: {}", e),
    };
    let mut remote = match repo.find_remote("origin") {
        Ok(remote) => remote,
        Err(e) => panic!("No origin remote found: {}", e),
    };
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_, _, _| {
        let username = "your_username";
        let password = "your_password";

        Cred::userpass_plaintext(username, password)
    });

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    let head = repo.head().unwrap();
    let branch_name = head.name().unwrap();

    let refspec = format!("{}:{}", branch_name, branch_name);
    let push_refs = vec![&refspec];


    match remote.push(&push_refs, Some(&mut push_options)) {
        Ok(remote) => remote,
        Err(e) => panic!("No origin remote found: {}", e),
    };

    print!("Donezo!!!!");

    match branch_name {
        "main" | "master" => {
            print!("On master!!!")
        }
        _ => {
            print!("On {}", branch_name.bold())
        }
    }
}