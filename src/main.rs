use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use encoding_rs::GBK;
use indicatif::{ProgressBar, ProgressStyle};

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(arg) = args.get(1) else {
        eprintln!("用法: peel <archive.zip|archive.7z>");
        eprintln!("  解压到同名新目录（目录已存在则自动加编号，绝不覆盖）");
        std::process::exit(2);
    };
    let archive = PathBuf::from(arg);
    if !archive.is_file() {
        eprintln!("文件不存在: {}", archive.display());
        std::process::exit(1);
    }
    let dest = dest_dir(&archive);
    std::fs::create_dir_all(&dest).expect("failed to create destination dir");

    let result = if is_7z(&archive) {
        extract_7z(&archive, &dest)
    } else {
        extract_zip(&archive, &dest)
    };

    match result {
        Ok(()) => println!("已解压到 {}", dest.display()),
        Err(e) => {
            eprintln!("解压失败: {e}");
            // Pause so a double-clicked console window doesn't vanish instantly.
            eprintln!("按回车键退出...");
            let _ = io::stdin().read_line(&mut String::new());
            std::process::exit(1);
        }
    }
}
