# chardet

[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/ljh-sh/chardet/badge)](https://scorecard.dev/)
[![CI](https://github.com/ljh-sh/chardet/actions/workflows/ci.yml/badge.svg)](https://github.com/ljh-sh/chardet/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

> Detect text file encoding via chardetng — BOM-aware, single-binary Rust CLI.

**chardet** reads a file (or stdin) and prints one line: the detected encoding and the path. It checks for a BOM first, then falls back to the chardetng heuristic on a bounded sample. Output is the lowercase WHATWG label (`utf-8`, `shift_jis`, `gbk`, ...).

## For AI agents

Minimal context with maximum flexibility — paste this one-line prompt into Claude Code, Cursor, or any agent's system prompt:

```md
Use `chardet <file>` to detect a text file's encoding. Install if missing: `cargo install chardet`. Output: `<encoding>\t<path>` per line.
```

## Install

### Cargo (recommended)

```bash
cargo install chardet
```

### Direct binary

```bash
curl -L https://github.com/ljh-sh/chardet/releases/latest/download/chardet-x86_64-unknown-linux-musl.tar.xz | tar xJ -
sudo mv chardet-x86_64-unknown-linux-musl/bin/chardet /usr/local/bin/
```

See the [releases page](https://github.com/ljh-sh/chardet/releases) for all targets (Linux musl/glibc, Windows, macOS).

### Build from source

Requires Rust 1.74+.

```bash
git clone https://github.com/ljh-sh/chardet
cd chardet
cargo build --release
```

## Usage

```sh
chardet file.txt
# utf-8	file.txt

chardet a.txt b.txt
# utf-8	a.txt
# shift_jis	b.txt

chardet < file.txt          # stdin (path shown as `-`)
cat file.txt | chardet

chardet ./src               # walk a directory, one line per regular file

chardet --no-bom file.txt   # ignore BOM, always run the heuristic
```

### Output format

One line per input, tab-separated:

```
<encoding>\t<path>
```

`<encoding>` is the lowercase [WHATWG encoding label](https://encoding.spec.whatwg.org/) (`utf-8`, `utf-16le`, `shift_jis`, `gbk`, `windows-1252`, ...).

### Options

```
-h, --help       show help
-V, --version    show version
--no-bom         ignore BOM, always run the heuristic
```

### Exit status

- `0` — all inputs detected successfully
- `1` — one or more inputs could not be read (errors go to stderr; detection continues for the remaining inputs)

## How it works

1. **BOM check** — if the file starts with a UTF-8 / UTF-16 / UTF-32 BOM, that encoding wins immediately (UTF-32 is checked before UTF-16 since they share a 2-byte prefix).
2. **ASCII fast path** — pure ASCII is reported as `utf-8`.
3. **chardetng heuristic** — the first 2 KB are fed to chardetng, which returns a single best-guess encoding name. ISO-2022-JP is disabled by default (safer for arbitrary input); UTF-8 is allowed.

The sample is bounded so large files don't get read into memory wholesale. BOM detection needs only the first 4 bytes.

## Limitations (v0)

- Encoding name only — no confidence score (planned for v1).
- No JSON output.
- Detection is heuristic; ambiguous short samples can be misidentified. BOM-prefixed files are always correct.
- Symlinks are skipped in directory mode.

## License

[Apache-2.0](LICENSE). Built on [chardetng](https://crates.io/crates/chardetng) (Apache-2.0 OR MIT) by Henri Sivonen.
