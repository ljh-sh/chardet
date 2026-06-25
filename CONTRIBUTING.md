# Contributing to chardet

Thanks for your interest! chardet is a small, focused tool. Please read this short guide before opening an issue or PR.

## Reporting issues

Open a [GitHub issue](../../issues) and include:

- Operating system and version
- chardet version (`chardet --version`)
- Installation method (cargo / binary / source)
- The exact command you ran and the input file (or a minimal sample)
- Expected vs actual output

## Feature requests

chardet deliberately stays small. It detects text encoding and prints a label per input. If your idea fits that scope, open an issue and explain the use case. Out-of-scope ideas (image/audio MIME detection, confidence scores) belong in other tools.

## Building from source

Requires Rust 1.74+.

```sh
git clone https://github.com/ljh-sh/chardet
cd chardet
cargo build --release
```

The binary will be at `target/release/chardet`.

## Running tests

```sh
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
```

## Pull requests

- Keep the change minimal and focused.
- Follow the existing Rust style.
- Update README examples if your change affects CLI behavior.
- Do not add heavy dependencies.

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 License.
