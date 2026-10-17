use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Destination dir: same folder as the archive, named after the archive stem.
pub fn dest_dir(archive: &Path) -> PathBuf {
    archive.with_extension("")
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
    eprint!("This archive is password protected. Enter password: ");
    match rpassword::read_password() {
        Ok(p) if p.is_empty() => None,
        Ok(p) => Some(p),
        Err(_) => None,
    }
}

fn extract_zip(path: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let Some(name) = entry.enclosed_name().map(|p| p.to_owned()) else {
            return Err(format!("unsafe path in archive: {}", entry.name()));
        };
        let out_path = dest.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = File::create(&out_path).map_err(|e| e.to_string())?;
            io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn extract_7z(path: &Path, dest: &Path) -> Result<(), String> {
    let try_extract = |pw: Option<&str>| -> Result<(), sevenz_rust::Error> {
        sevenz_rust::decompress_file_with_password(path, dest, pw.unwrap_or(""))
    };
    try_extract(None)
        .or_else(|e1| {
            let pw = ask_password();
            match pw {
                Some(p) => try_extract(Some(&p))
                    .map_err(|e2| format!("wrong password or failed: {e1}; retry failed: {e2}")),
                None => Err(format!("password required: {e1}")),
            }
        })
        .map_err(|e| e.to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(arg) = args.get(1) else {
        eprintln!("Usage: peel <archive.zip|archive.7z>");
        std::process::exit(2);
    };
    let archive = PathBuf::from(arg);
    if !archive.is_file() {
        eprintln!("Not a file: {}", archive.display());
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
        Ok(()) => println!("Extracted to {}", dest.display()),
        Err(e) => {
            eprintln!("Failed: {e}");
            // Pause so a double-clicked console window doesn't vanish instantly.
            eprintln!("Press Enter to exit...");
            let _ = io::stdin().read_line(&mut String::new());
            std::process::exit(1);
        }
    }
}
