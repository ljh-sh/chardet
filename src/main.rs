// chardet — detect text file encoding via chardetng.
//
// Strategy:
//   1. BOM check first (UTF-8/16/32, with and without endianness).
//   2. Otherwise feed a bounded sample (up to SAMPLE_BYTES) to chardetng.
//
// Output: one line per input, `<encoding>\t<path>`, encoding name is the
// encoding_rs WHATWG label (lowercase, e.g. `utf-8`, `shift_jis`).

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SAMPLE_BYTES: usize = 2048;

const USAGE: &str = "\
chardet 0.1.0 — detect text file encoding via chardetng

Usage:
  chardet <file>...            detect encoding of each file
  chardet -                    read from stdin (path shown as `-`)
  chardet                      read from stdin (path shown as `-`)

Output:
  One line per input: `<encoding>\t<path>` (tab-separated).
  Encoding is the WHATWG label from encoding_rs (utf-8, shift_jis, ...).

Options:
  -h, --help       show this help
  -V, --version    show version
  --no-bom         ignore BOM, always run the heuristic

Exit status:
  0  all inputs detected successfully
  1  one or more inputs could not be read
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut no_bom = false;
    let mut paths: Vec<String> = Vec::new();

    for a in &args {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("chardet 0.1.0");
                return ExitCode::SUCCESS;
            }
            "--no-bom" => no_bom = true,
            _ => paths.push(a.clone()),
        }
    }

    // No path args (or a single `-`) means stdin.
    let stdin_mode = paths.is_empty() || paths.iter().any(|p| p == "-");
    if stdin_mode && paths.iter().all(|p| p == "-") {
        paths = vec!["-".to_string()];
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut had_error = false;

    for p in &paths {
        match detect_one(p, no_bom) {
            Ok(enc) => {
                // For directories detect_one already printed each child; the
                // returned label is the sentinel "directory" which we skip.
                if enc != "directory" {
                    let _ = writeln!(out, "{}\t{p}", lower(enc));
                }
            }
            Err(e) => {
                let _ = writeln!(io::stderr(), "chardet: {p}: {e}");
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

/// Detect the encoding of a single input. `path` is `-` for stdin.
/// For directories, prints each child inline and returns the sentinel
/// "directory" label so the caller knows not to print the path itself.
fn detect_one(path: &str, no_bom: bool) -> Result<&'static str, String> {
    if path == "-" {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;
        Ok(detect_bytes(&buf, no_bom))
    } else {
        let p = Path::new(path);
        if !p.exists() {
            return Err("no such file or directory".to_string());
        }
        if p.is_dir() {
            detect_dir(p, no_bom).map(|_| "directory")
        } else {
            let data = fs::read(p).map_err(|e| e.to_string())?;
            Ok(detect_bytes(&data, no_bom))
        }
    }
}

/// Walk a directory and print one line per regular file. Returns Ok(()) on
/// completion; per-file read errors are reported to stderr and don't abort.
fn detect_dir(dir: &Path, no_bom: bool) -> Result<(), String> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut files: Vec<PathBuf> = Vec::new();
    walk(dir, &mut files).map_err(|e| e.to_string())?;
    files.sort();
    for f in &files {
        match fs::read(f) {
            Ok(data) => {
                let enc = detect_bytes(&data, no_bom);
                let _ = writeln!(out, "{}\t{}", lower(enc), f.display());
            }
            Err(e) => {
                let _ = writeln!(io::stderr(), "chardet: {}: {}", f.display(), e);
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

/// Core detection on a byte buffer. Returns a static encoding_rs name.
fn detect_bytes(data: &[u8], no_bom: bool) -> &'static str {
    if !no_bom {
        if let Some(enc) = detect_bom(data) {
            return enc;
        }
    }

    // Pure ASCII is a subset of every legacy encoding and of UTF-8; report
    // it as utf-8 explicitly so callers get a stable label.
    if data.is_ascii() {
        return "utf-8";
    }

    let sample = sample(data, SAMPLE_BYTES);
    let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
    detector.feed(sample, true);
    let enc = detector.guess(None, chardetng::Utf8Detection::Allow);
    enc.name()
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
    fn ascii_is_utf8() {
        assert_eq!(detect_bytes(b"hello world\n", false), "utf-8");
    }

    #[test]
    fn bom_utf8() {
        assert_eq!(detect_bytes(b"\xef\xbb\xbfhi", false), "utf-8");
    }

    #[test]
    fn bom_utf16be() {
        assert_eq!(detect_bytes(&[0xfe, 0xff, 0x00, b'H'], false), "utf-16be");
    }

    #[test]
    fn bom_utf16le() {
        assert_eq!(detect_bytes(&[0xff, 0xfe, b'H', 0x00], false), "utf-16le");
    }

    #[test]
    fn bom_utf32le_before_utf16le() {
        // FF FE 00 00 must read as utf-32le, not utf-16le.
        assert_eq!(detect_bytes(&[0xff, 0xfe, 0x00, 0x00], false), "utf-32le");
    }

    #[test]
    fn bom_utf32be() {
        assert_eq!(detect_bytes(&[0x00, 0x00, 0xfe, 0xff], false), "utf-32be");
    }

    #[test]
    fn no_bom_flag_ignores_bom() {
        // With --no-bom, the heuristic runs; "hi" still decodes as utf-8
        // (chardetng returns the canonical-case name "UTF-8").
        assert!(detect_bytes(b"\xef\xbb\xbfhi", true).eq_ignore_ascii_case("utf-8"));
    }

    #[test]
    fn shift_jis_from_literal() {
        // "こんにちは" in Shift_JIS, no BOM.
        let bytes = [0x82, 0xb1, 0x82, 0xf1, 0x82, 0xc9, 0x82, 0xbf, 0x82, 0xcd];
        let enc = detect_bytes(&bytes, false);
        // chardetng reports a Japanese legacy encoding; accept the family.
        assert!(
            enc.eq_ignore_ascii_case("Shift_JIS")
                || enc.eq_ignore_ascii_case("windows-31j")
                || enc.eq_ignore_ascii_case("EUC-JP"),
            "got {enc}"
        );
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
}
