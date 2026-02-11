//! LFS (Large File Storage) module
//!
//! Provides functionality for storing large files in cloud storage (AWS S3)
//! while keeping only pointer files in git.

pub mod cache;
pub mod config;
pub mod pointer;
pub mod scanner;
pub mod storage;

pub use cache::Cache;
pub use config::LfsConfig;
pub use pointer::Pointer;
pub use scanner::Scanner;
