// chardet — detect text file encoding via chardetng.
//
// Strategy:
//   1. BOM check first (UTF-8/16/32, with and without endianness).
//   2. Otherwise feed a bounded sample (up to SAMPLE_BYTES) to chardetng.
//
// Output: one line per input, `<encoding>\t<confidence>\t<path>`, where the
// encoding is the encoding_rs WHATWG label (lowercased, e.g. `utf-8`,
// `shift_jis`) and `<confidence>` is 1 when the result is certain (a BOM, pure
// ASCII, or chardetng reporting confidence) and 0 otherwise. chardetng exposes
// only a boolean confidence — there is no numeric score to report.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SAMPLE_BYTES: usize = 2048;

const USAGE: &str = "\
chardet 0.2.0 — detect text file encoding via chardetng

Usage:
  chardet <file>...                  detect encoding of each file
  chardet -                          read file content from stdin (path `-`)
  chardet                            read file content from stdin (path `-`)
  chardet --files-from <PATH|->      detect each path listed in a file or stdin

Output:
  One line per input, tab-separated: `<encoding>\t<confidence>\t<path>`.
  <encoding>   WHATWG label (utf-8, shift_jis, ...), lowercased.
  <confidence> 1 if the result is certain (BOM / pure ASCII / chardetng
               confident), else 0. chardetng is boolean — no numeric score.
  <path>       the path as given (`-` means stdin content).

Options:
  -h, --help               show this help
  -V, --version            show version
  --no-bom                 ignore BOM, always run the heuristic
  --files-from <PATH|->    detect each path from a newline-separated list in
                           <PATH>; use `-` for stdin
  -0, --null               with --files-from, paths are NUL-separated

Exit status:
  0  all inputs detected successfully
  1  one or more inputs could not be read
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut no_bom = false;
    let mut nul = false;
    let mut files_from: Option<String> = None;
    let mut paths: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("chardet 0.2.0");
                return ExitCode::SUCCESS;
            }
            "--no-bom" => no_bom = true,
            "-0" | "--null" => nul = true,
            "--files-from" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("chardet: --files-from requires an argument");
                    return ExitCode::FAILURE;
                }
                files_from = Some(args[i].clone());
            }
            s if s.starts_with("--files-from=") => {
                files_from = Some(s["--files-from=".len()..].to_string());
            }
            _ => paths.push(args[i].clone()),
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut had_error = false;

    if let Some(src) = files_from {
        match read_file_list(&src, nul) {
            Ok(list) => {
                for p in &list {
                    if let Err(e) = detect_path(p, no_bom, &mut out) {
                        eprintln!("chardet: {p}: {e}");
                        had_error = true;
                    }
                }
            }
            Err(e) => {
                eprintln!("chardet: --files-from {src}: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        // No path args (or a single `-`) means read file content from stdin.
        let stdin_mode = paths.is_empty() || paths.iter().any(|p| p == "-");
        if stdin_mode && paths.iter().all(|p| p == "-") {
            paths = vec!["-".to_string()];
        }
        for p in &paths {
            if let Err(e) = detect_one(p, no_bom, &mut out) {
                eprintln!("chardet: {p}: {e}");
                had_error = true;
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Read a list of paths from a file or stdin (`src == "-"`). Entries are split
/// on newlines, or on NUL bytes when `nul` is set, trimmed, and empties
/// dropped.
fn read_file_list(src: &str, nul: bool) -> Result<Vec<String>, String> {
    let raw: Vec<u8> = if src == "-" {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;
        buf
    } else {
        fs::read(src).map_err(|e| e.to_string())?
    };
    let sep: u8 = if nul { 0 } else { b'\n' };
    Ok(raw
        .split(|&b| b == sep)
        .map(|chunk| String::from_utf8_lossy(chunk).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Detect one input. `-` reads file content from stdin; directories are walked
/// and each child is printed inline.
fn detect_one<W: Write>(path: &str, no_bom: bool, out: &mut W) -> Result<(), String> {
    if path == "-" {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;
        let (enc, conf) = detect_bytes(&buf, no_bom);
        write_line(out, enc, conf, "-");
        Ok(())
    } else {
        detect_path(path, no_bom, out)
    }
}

/// Detect a concrete filesystem path (never stdin). Directories are walked.
fn detect_path<W: Write>(path: &str, no_bom: bool, out: &mut W) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err("no such file or directory".to_string());
    }
    if p.is_dir() {
        detect_dir(p, no_bom, out)
    } else {
        let data = fs::read(p).map_err(|e| e.to_string())?;
        let (enc, conf) = detect_bytes(&data, no_bom);
        write_line(out, enc, conf, path);
        Ok(())
    }
}

/// Walk a directory and print one line per regular file. Per-file read errors
/// are reported to stderr and do not abort the walk.
fn detect_dir<W: Write>(dir: &Path, no_bom: bool, out: &mut W) -> Result<(), String> {
    let mut files: Vec<PathBuf> = Vec::new();
    walk(dir, &mut files).map_err(|e| e.to_string())?;
    files.sort();
    for f in &files {
        match fs::read(f) {
            Ok(data) => {
                let (enc, conf) = detect_bytes(&data, no_bom);
                write_line(out, enc, conf, &f.display().to_string());
            }
            Err(e) => {
                eprintln!("chardet: {}: {}", f.display(), e);
            }
        }
    }
    Ok(())
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walk(&p, out)?;
        } else if ft.is_file() {
            out.push(p);
        }
        // symlinks and other types are skipped.
    }
    Ok(())
}

/// Emit one output line: `<encoding>\t<confidence>\t<path>`.
fn write_line<W: Write>(out: &mut W, enc: &str, conf: u8, path: &str) {
    let _ = writeln!(out, "{}\t{conf}\t{path}", lower(enc));
}

/// Core detection on a byte buffer. Returns the encoding_rs name and a
/// confidence flag: 1 = certain (BOM / pure ASCII / chardetng confident),
/// 0 = best-effort guess.
fn detect_bytes(data: &[u8], no_bom: bool) -> (&'static str, u8) {
    if !no_bom {
        if let Some(enc) = detect_bom(data) {
            return (enc, 1);
        }
    }

    // Pure ASCII is a subset of every legacy encoding and of UTF-8; report it
    // as utf-8 explicitly so callers get a stable, certain label.
    if data.is_ascii() {
        return ("utf-8", 1);
    }

    let sample = sample(data, SAMPLE_BYTES);
    let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
    // `feed` returns chardetng's only confidence signal: whether it is already
    // confident enough that further input would not change the answer.
    let confident = detector.feed(sample, true);
    let enc = detector.guess(None, chardetng::Utf8Detection::Allow);
    (enc.name(), u8::from(confident))
}

/// BOM detection. Returns the encoding_rs name, or None when no BOM.
fn detect_bom(data: &[u8]) -> Option<&'static str> {
    // UTF-32 BOMs must be checked before UTF-16 (both share 2-byte prefix).
    if data.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
        return Some("utf-32be");
    }
    if data.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
        return Some("utf-32le");
    }
    if data.starts_with(&[0xFE, 0xFF]) {
        return Some("utf-16be");
    }
    if data.starts_with(&[0xFF, 0xFE]) {
        return Some("utf-16le");
    }
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Some("utf-8");
    }
    None
}

fn sample(data: &[u8], limit: usize) -> &[u8] {
    if data.len() <= limit {
        return data;
    }
    &data[..limit]
}

/// Lowercase an encoding name. encoding_rs names are ASCII WHATWG labels, so
/// `to_ascii_lowercase` is allocation-only-when-needed and locale-independent.
fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_utf8_certain() {
        let (enc, conf) = detect_bytes(b"hello world\n", false);
        assert_eq!(enc, "utf-8");
        assert_eq!(conf, 1);
    }

    #[test]
    fn bom_utf8_certain() {
        let (enc, conf) = detect_bytes(b"\xef\xbb\xbfhi", false);
        assert_eq!(enc, "utf-8");
        assert_eq!(conf, 1);
    }

    #[test]
    fn bom_utf16be() {
        let (enc, conf) = detect_bytes(&[0xfe, 0xff, 0x00, b'H'], false);
        assert_eq!(enc, "utf-16be");
        assert_eq!(conf, 1);
    }

    #[test]
    fn bom_utf16le() {
        let (enc, _) = detect_bytes(&[0xff, 0xfe, b'H', 0x00], false);
        assert_eq!(enc, "utf-16le");
    }

    #[test]
    fn bom_utf32le_before_utf16le() {
        // FF FE 00 00 must read as utf-32le, not utf-16le.
        let (enc, _) = detect_bytes(&[0xff, 0xfe, 0x00, 0x00], false);
        assert_eq!(enc, "utf-32le");
    }

    #[test]
    fn bom_utf32be() {
        let (enc, _) = detect_bytes(&[0x00, 0x00, 0xfe, 0xff], false);
        assert_eq!(enc, "utf-32be");
    }

    #[test]
    fn no_bom_flag_ignores_bom() {
        // With --no-bom, the heuristic runs; "hi" still decodes as utf-8.
        let (enc, _) = detect_bytes(b"\xef\xbb\xbfhi", true);
        assert!(enc.eq_ignore_ascii_case("utf-8"));
    }

    #[test]
    fn shift_jis_from_literal() {
        // "こんにちは" in Shift_JIS, no BOM.
        let bytes = [0x82, 0xb1, 0x82, 0xf1, 0x82, 0xc9, 0x82, 0xbf, 0x82, 0xcd];
        let (enc, conf) = detect_bytes(&bytes, false);
        // chardetng reports a Japanese legacy encoding; accept the family.
        assert!(
            enc.eq_ignore_ascii_case("Shift_JIS")
                || enc.eq_ignore_ascii_case("windows-31j")
                || enc.eq_ignore_ascii_case("EUC-JP"),
            "got {enc}"
        );
        // chardetng is boolean — heuristic confidence is always 0 or 1.
        assert!(conf == 0 || conf == 1, "conf out of range: {conf}");
    }

    #[test]
    fn confidence_is_always_boolean() {
        // Any non-BOM, non-ASCII heuristic result must still be 0 or 1.
        let (_, conf) = detect_bytes("café münchën".as_bytes(), false);
        assert!(conf == 0 || conf == 1);
    }

    #[test]
    fn sample_short_returns_all() {
        assert_eq!(sample(b"abc", 10), b"abc");
    }

    #[test]
    fn sample_long_truncates() {
        let big = vec![b'x'; 100];
        assert_eq!(sample(&big, 10).len(), 10);
    }

    #[test]
    fn read_file_list_splits_on_newline_or_nul() {
        let dir = std::env::temp_dir().join(format!("chardet-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let f = dir.join("list.txt");

        // Newline-separated; blank lines are dropped and entries trimmed.
        fs::write(&f, "a.txt\n\n  b.txt  \n").unwrap();
        let list = read_file_list(f.to_str().unwrap(), false).unwrap();
        assert_eq!(list, vec!["a.txt".to_string(), "b.txt".to_string()]);

        // NUL-separated when `nul` is set.
        fs::write(&f, b"x\0y\0").unwrap();
        let list = read_file_list(f.to_str().unwrap(), true).unwrap();
        assert_eq!(list, vec!["x".to_string(), "y".to_string()]);

        let _ = fs::remove_file(&f);
    }
}
