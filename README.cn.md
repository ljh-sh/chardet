# chardet

[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/ljh-sh/chardet/badge)](https://scorecard.dev/)
[![CI](https://github.com/ljh-sh/chardet/actions/workflows/ci.yml/badge.svg)](https://github.com/ljh-sh/chardet/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

> 用 chardetng 检测文本文件编码 —— BOM 优先、单二进制 Rust CLI。

**chardet** 读取文件（或 stdin）并输出一行：检测到的编码 + 路径。先查 BOM，没有 BOM 就在有限样本上跑 chardetng 启发式。输出为小写的 WHATWG 标签（`utf-8`、`shift_jis`、`gbk`……）。

## 给 AI agent

把下面一行塞进 Claude Code / Cursor 等任何 agent 的 system prompt 即可：

```md
用 `chardet <file>` 检测文本文件编码。缺失则安装：`cargo install chardet`。输出格式：每行 `<encoding>\t<path>`。
```

## 安装

### Cargo（推荐）

```bash
cargo install chardet
```

### 直接下载二进制

```bash
curl -L https://github.com/ljh-sh/chardet/releases/latest/download/chardet-x86_64-unknown-linux-musl.tar.xz | tar xJ -
sudo mv chardet-x86_64-unknown-linux-musl/bin/chardet /usr/local/bin/
```

全部平台见 [releases 页面](https://github.com/ljh-sh/chardet/releases)（Linux musl/glibc、Windows、macOS）。

### 源码构建

需要 Rust 1.74+。

```bash
git clone https://github.com/ljh-sh/chardet
cd chardet
cargo build --release
```

## 用法

```sh
chardet file.txt
# utf-8	file.txt

chardet a.txt b.txt
# utf-8	a.txt
# shift_jis	b.txt

chardet < file.txt          # 从 stdin 读（路径显示为 `-`）
cat file.txt | chardet

chardet ./src               # 遍历目录，每个普通文件一行

chardet --no-bom file.txt   # 忽略 BOM，强制跑启发式
```

### 输出格式

每个输入一行，Tab 分隔：

```
<encoding>\t<path>
```

`<encoding>` 是小写的 [WHATWG 编码标签](https://encoding.spec.whatwg.org/)（`utf-8`、`utf-16le`、`shift_jis`、`gbk`、`windows-1252`……）。

### 选项

```
-h, --help       显示帮助
-V, --version    显示版本
--no-bom         忽略 BOM，强制跑启发式
```

### 退出码

- `0` —— 所有输入都成功检测
- `1` —— 有输入读不出来（错误打到 stderr，其余输入继续检测）

## 工作原理

1. **BOM 检查** —— 文件以 UTF-8 / UTF-16 / UTF-32 BOM 开头就直接定编码（UTF-32 在 UTF-16 之前检查，因为前 2 字节相同）。
2. **ASCII 快速路径** —— 纯 ASCII 直接报 `utf-8`。
3. **chardetng 启发式** —— 取前 2 KB 喂给 chardetng，返回一个最佳猜测的编码名。默认禁用 ISO-2022-JP（对任意输入更安全），允许 UTF-8。

样本有上限，大文件不会被整块读进内存。BOM 检测只需前 4 字节。

## 限制（v0）

- 只输出编码名，没有置信度（v1 规划）。
- 没有 JSON 输出。
- 检测是启发式的，短样本可能误判；带 BOM 的文件一定准确。
- 目录模式下跳过符号链接。

## License

[Apache-2.0](LICENSE)。基于 Henri Sivonen 的 [chardetng](https://crates.io/crates/chardetng)（Apache-2.0 OR MIT）。
