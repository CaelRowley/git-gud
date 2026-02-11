use std::process::{Command, Stdio};

/// Pass command directly to git, preserving colors and interactivity.
/// This is the primary way to delegate unknown commands to git.
pub fn passthrough(args: &[String]) -> i32 {
    let result = Command::new("git")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .and_then(|mut child| child.wait());

    match result {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("gg: failed to run git: {}", e);
            1
        }
    }
}

/// Run git command with string slice args (convenience wrapper).
/// Use this for internal git calls where you don't need to capture output.
pub fn run(args: &[&str]) -> i32 {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    passthrough(&args)
}

/// Run a sequence of git commands, stopping on first failure.
#[allow(dead_code)]
pub fn run_sequence(commands: &[&[&str]]) -> i32 {
    for cmd in commands {
        let code = run(cmd);
        if code != 0 {
            return code;
        }
    }
    0
}

/// Run git and capture output (when you need to process it).
/// Note: This loses colors, only use when you need to parse the output.
pub fn capture(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_git_version() {
        let result = capture(&["--version"]);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("git version"));
    }

    #[test]
    fn test_capture_invalid_command() {
        let result = capture(&["not-a-real-command-12345"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_returns_zero_on_success() {
        // git --version should succeed
        let code = run(&["--version"]);
        assert_eq!(code, 0);
    }

    #[test]
    fn test_run_returns_nonzero_on_failure() {
        // Invalid git command should fail
        let code = run(&["not-a-real-command-12345"]);
        assert_ne!(code, 0);
    }

    #[test]
    fn test_run_sequence_stops_on_failure() {
        let commands: &[&[&str]] = &[
            &["--version"],                  // succeeds
            &["not-a-real-command-12345"],   // fails
            &["--version"],                  // should not run
        ];
        let code = run_sequence(commands);
        assert_ne!(code, 0);
    }

    #[test]
    fn test_run_sequence_all_succeed() {
        let commands: &[&[&str]] = &[
            &["--version"],
            &["--version"],
        ];
        let code = run_sequence(commands);
        assert_eq!(code, 0);
    }
}
