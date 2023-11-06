use anyhow::{Context, Result};
use byte_unit::Byte;
use chrono::{Duration, Local, TimeZone};
use clap::Parser;
use colored::*;
use std::fs::{self, Metadata};
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
#[cfg(target_os = "linux")]
use users::{get_group_by_gid, get_user_by_uid};

#[derive(Parser, Debug)]
#[command(about)]
struct Args {
    /// files to read
    #[arg()]
    files: Vec<PathBuf>,

    /// do not ignore entries starting with .
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// list directories themselves, not their content
    #[arg(short = 'd', long = "directory")]
    directory: bool,

    /// use a long listing format
    #[arg(short = 'l')]
    long: bool,

    /// print sizes exactly in bytes
    #[arg(short = 'n', long = "not-human-readable")]
    not_human_readable: bool,
}

fn main() {
    let mut args = Args::parse();

    #[cfg(target_os = "windows")]
    {
        println!("Running on Windows");
    }

    if args.files.is_empty() {
        args.files.push(PathBuf::from("."));
    }

    args.files.sort_unstable();

    if let Err(e) = list_files(&args) {
        eprintln!("{} {:?}", "ls - error:".red(), e);
        std::process::exit(1);
    }
}

/// Go through files in args.files printing info about them.
///
/// If the file is a directory, info for each entry is printed.
fn list_files(args: &Args) -> Result<()> {
    for filename in &args.files {
        if filename.is_dir() && !args.directory {
            println!("{}:", filename.to_str().unwrap_or_default().blue().bold());

            let mut entries: Vec<PathBuf> = fs::read_dir(filename)
                .context(format!("Failed to read dir {}", filename.display()))?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .collect();
            entries.sort_unstable();

            for path in entries {
                file_info(args, &path)?;
            }
        } else {
            file_info(args, filename)?;
        }
    }

    Ok(())
}

/// Print info about file specified by path,
/// according to flags from args.
fn file_info(args: &Args, path: &Path) -> Result<()> {
    let filename = if path == PathBuf::from(".") {
        "."
    } else if path == PathBuf::from("..") {
        ".."
    } else {
        path.file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
    };

    let metadata = fs::symlink_metadata(path).context(format!("cannot acces '{}'", filename))?;

    if !args.all && filename != "." && filename != ".." && filename.starts_with('.') {
        // Skip hidden files
        return Ok(());
    }

    let suffix = if path.is_dir() { "/" } else { "" };

    if !args.long {
        println!("{}{}", filename, suffix);
        return Ok(());
    }

    let size = if args.not_human_readable {
        metadata.len().to_string()
    } else {
        let bytes = Byte::from_bytes(metadata.len() as u128);
        bytes.get_appropriate_unit(true).to_string()
    };

    let modified = get_modified_str(&metadata)?;
    let file_type = get_file_type_str(&metadata);

    let target_file = if path.is_symlink() {
        format!(" -> {}", fs::read_link(path)?.to_str().unwrap_or_default())
    } else {
        String::from("")
    };

    #[cfg(target_os = "windows")]
    {
        println!(
            "{} {:11} {} {}{}{}",
            file_type, size, modified, filename, suffix, target
        );
    }

    #[cfg(target_os = "linux")]
    {
        let mode = get_mode_str(metadata.mode());
        let nlink = metadata.nlink();
        let user = match get_user_by_uid(metadata.uid()) {
            Some(user) => user.name().to_string_lossy().to_string(),
            None => "Unknown".to_string(),
        };
        let group = match get_group_by_gid(metadata.gid()) {
            Some(group) => group.name().to_string_lossy().to_string(),
            None => "Unknown".to_string(),
        };

        println!(
            "{}{} {} {:8} {:8} {:>10} {} {}{}{}",
            file_type, mode, nlink, user, group, size, modified, filename, suffix, target_file
        );
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn get_mode_str(mode: u32) -> String {
    let mut mode_str = String::new();

    mode_str.push(if mode & 0o400 > 0 { 'r' } else { '-' });
    mode_str.push(if mode & 0o200 > 0 { 'w' } else { '-' });
    mode_str.push(if mode & 0o100 > 0 { 'x' } else { '-' });

    mode_str.push(if mode & 0o040 > 0 { 'r' } else { '-' });
    mode_str.push(if mode & 0o020 > 0 { 'w' } else { '-' });
    mode_str.push(if mode & 0o010 > 0 { 'x' } else { '-' });

    mode_str.push(if mode & 0o004 > 0 { 'r' } else { '-' });
    mode_str.push(if mode & 0o002 > 0 { 'w' } else { '-' });
    mode_str.push(if mode & 0o001 > 0 { 'x' } else { '-' });

    mode_str
}

fn get_file_type_str(metadata: &Metadata) -> String {
    if metadata.is_dir() {
        "d".to_string()
    } else if metadata.is_symlink() {
        "l".to_string()
    } else {
        "-".to_string()
    }
}

fn get_modified_str(metadata: &Metadata) -> Result<String> {
    let std_duration = metadata.modified()?.duration_since(UNIX_EPOCH)?;
    let duration = Duration::from_std(std_duration)?;
    let datetime = Local.timestamp_opt(duration.num_seconds(), 0).unwrap();
    Ok(datetime.format("%b %d %H:%M").to_string())
}

