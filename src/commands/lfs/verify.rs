//! Verify LFS storage configuration and connectivity

use crate::lfs::LfsConfig;
use aws_sdk_s3::Client;
use clap::Args;
use colored::Colorize;

#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Test write access by uploading a small test file
    #[arg(short, long)]
    pub write: bool,
}

/// Verify LFS configuration and S3 connectivity
pub fn run(args: VerifyArgs) -> i32 {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{} Failed to create async runtime: {}", "Error:".red().bold(), e);
            return 1;
        }
    };

    rt.block_on(async {
        match run_inner(args).await {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("{} {}", "Error:".red().bold(), e);
                1
            }
        }
    })
}

async fn run_inner(args: VerifyArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?;

    println!("{}", "Verifying LFS configuration...".cyan().bold());
    println!();

    // Step 1: Check config file exists
    print!("  {} Configuration file... ", "Checking".dimmed());
    let config = match LfsConfig::load(repo_root) {
        Ok(c) => {
            println!("{}", "OK".green());
            c
        }
        Err(e) => {
            println!("{}", "FAILED".red());
            return Err(format!(
                "Configuration not found: {}\n\nRun 'gg lfs install' to create a configuration file.",
                e
            ).into());
        }
    };

    // Step 2: Validate config values
    print!("  {} Configuration values... ", "Validating".dimmed());
    if let Err(e) = config.validate() {
        println!("{}", "FAILED".red());
        return Err(format!("Invalid configuration: {}", e).into());
    }
    println!("{}", "OK".green());

    // Display config summary
    println!();
    println!("  {}", "Configuration:".cyan());
    println!("    Provider: {:?}", config.storage.provider);
    println!("    Bucket:   {}", config.storage.bucket);
    println!("    Region:   {}", config.storage.region);
    if let Some(prefix) = &config.storage.prefix {
        println!("    Prefix:   {}", prefix);
    }
    if let Some(endpoint) = &config.storage.endpoint {
        println!("    Endpoint: {}", endpoint);
    }
    println!();

    // Step 3: Check AWS credentials
    print!("  {} AWS credentials... ", "Checking".dimmed());
    let aws_config = build_aws_config(&config).await;
    
    match aws_config.credentials_provider() {
        Some(_) => println!("{}", "OK".green()),
        None => {
            println!("{}", "WARNING".yellow());
            println!("    {}", "No credentials found. Options:".yellow());
            println!("    {}",   "  1. Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY env vars".yellow());
            println!("    {}",   "  2. Configure ~/.aws/credentials".yellow());
            println!("    {}",   "  3. Add [storage.credentials] to .gg/lfs.toml".yellow());
        }
    }

    // Step 4: Check bucket exists and is accessible
    print!("  {} Bucket access... ", "Checking".dimmed());
    let client = Client::new(&aws_config);
    
    match client
        .head_bucket()
        .bucket(&config.storage.bucket)
        .send()
        .await
    {
        Ok(_) => {
            println!("{}", "OK".green());
        }
        Err(e) => {
            println!("{}", "FAILED".red());
            let err_str = e.to_string();
            
            if err_str.contains("NoSuchBucket") || err_str.contains("404") {
                return Err(format!(
                    "Bucket '{}' does not exist.\n\nCreate the bucket in AWS console or update .gg/lfs.toml",
                    config.storage.bucket
                ).into());
            } else if err_str.contains("AccessDenied") || err_str.contains("403") {
                return Err(format!(
                    "Access denied to bucket '{}'.\n\nCheck your AWS credentials have s3:ListBucket permission.",
                    config.storage.bucket
                ).into());
            } else if err_str.contains("InvalidAccessKeyId") {
                return Err("Invalid AWS access key ID.\n\nCheck your credentials (env vars, ~/.aws/credentials, or [storage.credentials] in .gg/lfs.toml).".into());
            } else if err_str.contains("SignatureDoesNotMatch") {
                return Err("Invalid AWS secret access key.\n\nCheck your credentials (env vars, ~/.aws/credentials, or [storage.credentials] in .gg/lfs.toml).".into());
            } else if err_str.contains("timeout") || err_str.contains("Timeout") {
                return Err(format!(
                    "Connection timeout.\n\nCheck your network connection and region setting (current: {}).",
                    config.storage.region
                ).into());
            } else {
                return Err(format!("Failed to access bucket: {}", err_str).into());
            }
        }
    }

    // Step 5: Test write access if requested
    if args.write {
        print!("  {} Write access... ", "Testing".dimmed());
        
        let test_key = format!(
            "{}/.gg-lfs-verify-test",
            config.storage.prefix.as_deref().unwrap_or("").trim_end_matches('/')
        );
        let test_key = test_key.trim_start_matches('/');
        
        // Try to upload a small test object
        match client
            .put_object()
            .bucket(&config.storage.bucket)
            .key(test_key)
            .body(aws_sdk_s3::primitives::ByteStream::from_static(b"gg-lfs-verify-test"))
            .send()
            .await
        {
            Ok(_) => {
                // Clean up test object
                let _ = client
                    .delete_object()
                    .bucket(&config.storage.bucket)
                    .key(test_key)
                    .send()
                    .await;
                
                println!("{}", "OK".green());
            }
            Err(e) => {
                println!("{}", "FAILED".red());
                let err_str = e.to_string();
                
                if err_str.contains("AccessDenied") || err_str.contains("403") {
                    return Err(format!(
                        "Write access denied to bucket '{}'.\n\nCheck your AWS credentials have s3:PutObject permission.",
                        config.storage.bucket
                    ).into());
                } else {
                    return Err(format!("Failed to write to bucket: {}", err_str).into());
                }
            }
        }
    }

    println!();
    println!("{}", "All checks passed!".green().bold());
    
    if !args.write {
        println!("{}", "Tip: Use 'gg lfs verify --write' to also test write permissions.".dimmed());
    }

    Ok(())
}

/// Build AWS config from LFS config
async fn build_aws_config(config: &LfsConfig) -> aws_config::SdkConfig {
    let mut builder = aws_config::from_env()
        .region(aws_config::Region::new(config.storage.region.clone()));

    if let Some(endpoint) = &config.storage.endpoint {
        builder = builder.endpoint_url(endpoint);
    }

    if let Some(creds) = &config.storage.credentials {
        let credentials = aws_sdk_s3::config::Credentials::new(
            &creds.access_key_id,
            &creds.secret_access_key,
            None,
            None,
            "gg-lfs-config",
        );
        builder = builder.credentials_provider(credentials);
    }

    builder.load().await
}
