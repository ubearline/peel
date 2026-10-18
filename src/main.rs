use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser};
use encoding_rs::GBK;
use indicatif::{ProgressBar, ProgressStyle};
use zip::write::SimpleFileOptions;

/// Destination dir: same folder as the archive, named after the archive stem.
/// If it already exists, append a sequence number: movie.zip -> movie/, movie (2)/, ...
/// Flat extraction into the archive's own folder is intentionally NOT offered:
/// it risks clobbering existing files with one careless click.
pub fn dest_dir(archive: &Path) -> PathBuf {
    let base = archive.with_extension("");
    let parent = base.parent().unwrap_or(Path::new("."));
    let name = base
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "extract".into());
    let mut candidate = parent.join(&name);
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{name} ({n})"));
        n += 1;
    }
    candidate
}

fn is_7z(path: &Path) -> bool {
    // Trust extension first, then fall back to magic bytes.
    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("7z"))
        .unwrap_or(false)
    {
        return true;
    }
    let mut buf = [0u8; 6];
    File::open(path)
        .and_then(|mut f| f.read_exact(&mut buf))
        .ok()
        .map(|_| &buf == b"7z\xBC\xAF\x27\x1C")
        .unwrap_or(false)
}

fn ask_password() -> Option<String> {
    eprint!("该压缩包已加密，请输入密码: ");
    match rpassword::read_password() {
        Ok(p) if p.is_empty() => None,
        Ok(p) => Some(p),
        Err(_) => None,
    }
}

/// Decode a raw entry-name byte string: UTF-8 first, GBK fallback.
/// Legacy Chinese zip tools (WinRAR/好压/360zip) store names in GBK.
fn decode_name(raw: &[u8]) -> String {
    match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => GBK.decode(raw).0.into_owned(),
    }
}

/// Safe relative path from decoded name; rejects absolute paths and `..` escapes.
fn safe_rel_path(name: &str) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() {
        return None;
    }
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            std::path::Component::Normal(c) => out.push(c),
            std::path::Component::CurDir => {}
            _ => return None, // ParentDir / Prefix / RootDir -> unsafe
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn new_bar(total: u64) -> ProgressBar {
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta}) {msg}",
        )
        .unwrap()
        .progress_chars("█▓░"),
    );
    bar
}

fn extract_zip(path: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    // Peek each entry; encrypted entries error on plain by_index, which is our cue to ask.
    let mut needs_password = false;
    let mut total = 0u64;
    for i in 0..zip.len() {
        match zip.by_index(i) {
            Ok(e) => total += e.size(),
            Err(zip::result::ZipError::UnsupportedArchive(m)) => {
                if m.contains("Password") {
                    needs_password = true;
                    break;
                }
            }
            Err(_) => {}
        }
    }
    let password = if needs_password { ask_password() } else { None };
    if needs_password {
        // sizes are readable after decrypt with the right password
        total = 0;
        for i in 0..zip.len() {
            if let Ok(e) = zip.by_index_decrypt(i, password.as_ref().unwrap().as_bytes()) {
                total += e.size();
            }
        }
    }
    let bar = new_bar(total);
    for i in 0..zip.len() {
        let mut entry = match &password {
            Some(p) => zip
                .by_index_decrypt(i, p.as_bytes())
                .map_err(|e| e.to_string())?,
            None => zip.by_index(i).map_err(|e| e.to_string())?,
        };
        let raw = entry.name_raw().to_vec();
        let Some(rel) = safe_rel_path(&decode_name(&raw)) else {
            return Err(format!("unsafe path in archive: {}", entry.name()));
        };
        bar.set_message(rel.display().to_string());
        let out_path = dest.join(rel);
        if !entry.is_dir() && out_path.exists() {
            return Err(format!("目标已存在，跳过覆盖: {}", out_path.display()));
        }
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = File::create(&out_path).map_err(|e| e.to_string())?;
            let n = io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
            bar.inc(n);
        }
    }
    bar.finish_with_message("完成");
    Ok(())
}

fn extract_7z(path: &Path, dest: &Path) -> Result<(), String> {
    use sevenz_rust::{Error, Password, SevenZReader};
    let try_extract = |pw: Option<&str>| -> Result<(), Error> {
        let mut reader =
            SevenZReader::open(path, Password::from(pw.unwrap_or(""))).map_err(Error::from)?;
        let total: u64 = reader.archive().files.iter().map(|e| e.size()).sum();
        let bar = new_bar(total);
        reader
            .for_each_entries(|entry, reader| {
                let out_path = dest.join(entry.name());
                if !entry.is_directory() && out_path.exists() {
                    return Err(sevenz_rust::Error::other(format!(
                        "目标已存在，跳过覆盖: {}",
                        out_path.display()
                    )));
                }
                if entry.is_directory() {
                    std::fs::create_dir_all(&out_path)
                        .map_err(|e| sevenz_rust::Error::other(format!("mkdir failed: {e}")))?;
                } else {
                    if let Some(parent) = out_path.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| sevenz_rust::Error::other(format!("mkdir failed: {e}")))?;
                    }
                    bar.set_message(entry.name().to_string());
                    let mut out = File::create(&out_path)
                        .map_err(|e| sevenz_rust::Error::other(format!("create failed: {e}")))?;
                    let n = io::copy(reader, &mut out)
                        .map_err(|e| sevenz_rust::Error::other(format!("write failed: {e}")))?;
                    bar.inc(n);
                }
                Ok(true)
            })
            .map_err(|e| {
                bar.finish_and_clear();
                e
            })?;
        bar.finish_with_message("完成");
        Ok(())
    };

    // Only prompt for a password when the failure is actually password-related;
    // other errors (e.g. existing files) must fail loudly instead of asking.
    let is_password_err = |e: &Error| {
        matches!(
            e,
            Error::PasswordRequired
                | Error::MaybeBadPassword(_)
                | Error::ChecksumVerificationFailed
        )
    };
    match try_extract(None) {
        Ok(()) => Ok(()),
        Err(e) if is_password_err(&e) => {
            let pw = ask_password();
            match pw {
                Some(p) => try_extract(Some(&p)).map_err(|e2| format!("密码错误或解压失败: {e2}")),
                None => Err("password required".into()),
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Compress a folder recursively into <sibling>.zip.
/// Entries are stored relative to the folder's PARENT (so the zip contains
/// the folder itself as root, matching what "压缩到同名zip" users expect
/// when they later extract it). Existing .zip files are excluded everywhere
/// in the tree: the output must never fold itself (or sibling archives) in,
/// which would also risk recursive self-inclusion on re-compress.
/// If the target zip exists, a numbered name is used, never overwrite.
fn compress_zip(folder: &Path) -> Result<PathBuf, String> {
    let parent = folder
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let name = folder
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| format!("无效目录: {}", folder.display()))?;

    let mut target = parent.join(format!("{name}.zip"));
    let mut n = 2;
    while target.exists() {
        target = parent.join(format!("{name} ({n}).zip"));
        n += 1;
    }

    // Walk first so the progress bar reflects the real total up front.
    let mut files: Vec<PathBuf> = Vec::new();
    let mut stack = vec![folder.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?.path();
            let is_zip = entry
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("zip"))
                .unwrap_or(false);
            if is_zip {
                continue; // never include .zip files (incl. the output itself)
            }
            if entry.is_dir() {
                stack.push(entry);
            } else if entry.is_file() {
                files.push(entry);
            }
        }
    }

    let total: u64 = files
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum();
    let bar = new_bar(total);
    let file = File::create(&target).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut added_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();

    for path in &files {
        let rel = path.strip_prefix(&parent).map_err(|e| e.to_string())?;
        // Rebuild directory entries (deduped) so empty dirs survive the round trip.
        for ancestor in rel.ancestors().skip(1) {
            let dir_entry = ancestor.to_string_lossy().replace('\\', "/");
            if added_dirs.insert(dir_entry.clone()) && !dir_entry.is_empty() {
                zip.add_directory(dir_entry, options)
                    .map_err(|e| e.to_string())?;
            }
        }
        bar.set_message(rel.display().to_string());
        zip.start_file(rel.to_string_lossy().replace('\\', "/"), options)
            .map_err(|e| e.to_string())?;
        let mut f = File::open(path).map_err(|e| e.to_string())?;
        let n = io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
        bar.inc(n);
    }
    zip.finish().map_err(|e| e.to_string())?;
    bar.finish_with_message("完成");
    Ok(target)
}

fn fail(e: String) -> ! {
    eprintln!("{e}");
    // Pause so a double-clicked console window doesn't vanish instantly.
    eprintln!("按回车键退出...");
    let _ = io::stdin().read_line(&mut String::new());
    std::process::exit(1);
}

#[derive(clap::Parser)]
#[command(
    name = "peel",
    version,
    about = "极简压缩/解压工具",
    after_help = "裸路径（无动词）保持智能默认：压缩包→解压到同名目录，目录→压缩成 zip。"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// 不带动词时的裸路径（智能默认）
    path: Option<PathBuf>,
}

#[derive(clap::Subcommand)]
enum Cmd {
    /// 解压压缩包
    X {
        /// 压缩包路径（多选时逐个处理）
        #[arg(required = true)]
        archives: Vec<PathBuf>,
        /// 解压到压缩包所在目录（不建同名文件夹，条目扁平展开）
        #[arg(long)]
        here: bool,
    },
    /// 压缩目录
    C {
        /// 目录路径（多选时逐个处理）
        #[arg(required = true)]
        folders: Vec<PathBuf>,
        /// 目标格式（目前仅 zip）
        #[arg(short, long, value_enum, default_value = "zip")]
        format: Format,
    },
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum Format {
    Zip,
}

fn extract_one(path: &Path, here: bool) -> Result<(), String> {
    if here {
        let dest = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let result = if is_7z(path) {
            extract_7z(path, &dest)
        } else {
            extract_zip(path, &dest)
        };
        match result {
            Ok(()) => println!("已解压到 {}", dest.display()),
            Err(e) => return Err(format!("解压失败: {e}")),
        }
    } else {
        let dest = dest_dir(path);
        std::fs::create_dir_all(&dest).map_err(|e| format!("创建目录失败: {e}"))?;
        let result = if is_7z(path) {
            extract_7z(path, &dest)
        } else {
            extract_zip(path, &dest)
        };
        match result {
            Ok(()) => println!("已解压到 {}", dest.display()),
            Err(e) => return Err(format!("解压失败: {e}")),
        }
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Some(Cmd::X { archives, here }) => {
            for a in &archives {
                if !a.exists() {
                    fail(format!("文件不存在: {}", a.display()));
                }
                if let Err(e) = extract_one(a, here) {
                    fail(e);
                }
            }
        }
        Some(Cmd::C {
            folders,
            format: Format::Zip,
        }) => {
            for f in &folders {
                if !f.is_dir() {
                    fail(format!("不是目录: {}", f.display()));
                }
                match compress_zip(f) {
                    Ok(target) => println!("已压缩到 {}", target.display()),
                    Err(e) => fail(format!("压缩失败: {e}")),
                }
            }
        }
        None => {
            let Some(arg) = cli.path else {
                // Clap already printed usage via help; keep a friendly hint.
                Cli::command().print_help().ok();
                println!();
                std::process::exit(2);
            };
            let path = PathBuf::from(&arg);
            if !path.exists() {
                eprintln!("文件不存在: {}", path.display());
                std::process::exit(1);
            }
            if path.is_dir() {
                match compress_zip(&path) {
                    Ok(target) => println!("已压缩到 {}", target.display()),
                    Err(e) => fail(format!("压缩失败: {e}")),
                }
            } else if let Err(e) = extract_one(&path, false) {
                fail(e);
            }
        }
    }
}
