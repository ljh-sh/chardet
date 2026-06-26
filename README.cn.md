# chardet

[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/ljh-sh/chardet/badge)](https://scorecard.dev/)
[![CI](https://github.com/ljh-sh/chardet/actions/workflows/ci.yml/badge.svg)](https://github.com/ljh-sh/chardet/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

> 用 chardetng 检测文本文件编码 —— BOM 优先、单二进制 Rust CLI。

**chardet** 读取文件（或 stdin）并输出一行：检测到的编码 + 置信度标记 + 路径。先查 BOM，没有 BOM 就在有限样本上跑 chardetng 启发式。输出为小写的 WHATWG 标签加一个 1/0 置信度（`utf-8`、`shift_jis`、`gbk`……）。

## 给 AI agent

把下面一行塞进 Claude Code / Cursor 等任何 agent 的 system prompt 即可：

```md
用 `chardet <file>` 检测文本文件编码。缺失则安装：`cargo install chardet-cli`。输出格式：每行 `<encoding>\t<confidence>\t<path>`；`<confidence>` 为 `1` 表示确定（BOM / ASCII / chardetng 确信），否则 `0`。
```

## 安装

### Cargo（推荐）

```bash
cargo install chardet-cli
```

包名是 `chardet-cli`（crates.io 上 `chardet` 已被占用），安装后的二进制名仍是 **`chardet`**。

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
# utf-8	1	file.txt

chardet a.txt b.txt
# utf-8	1	a.txt
# shift_jis	1	b.txt

chardet < file.txt          # 从 stdin 读文件内容（路径显示为 `-`）
cat file.txt | chardet

chardet ./src               # 遍历目录，每个普通文件一行

chardet --no-bom file.txt   # 忽略 BOM，强制跑启发式

chardet --files-from list.txt         # 检测 list.txt 里列出的每个路径
git ls-files | chardet --files-from -            # ...或从 stdin 读路径列表
find . -print0 | chardet --files-from - -0       # NUL 分隔的路径
```

### 输出格式

每个输入一行，Tab 分隔：

```
<encoding>\t<confidence>\t<path>
```

- `<encoding>` —— 小写的 [WHATWG 编码标签](https://encoding.spec.whatwg.org/)（`utf-8`、`utf-16le`、`shift_jis`、`gbk`、`windows-1252`……）。
- `<confidence>` —— `1` 表示确定，否则 `0`。确定 = 存在 BOM / 纯 ASCII / chardetng 报告确信。chardetng 只提供布尔置信度，所以这一列是 1/0 标记，不是数值分数。
- `<path>` —— 给定的路径（从 stdin 读文件内容时为 `-`）。

### 选项

```
-h, --help               显示帮助
-V, --version            显示版本
--no-bom                 忽略 BOM，强制跑启发式
--files-from <PATH|->    检测 <PATH> 里换行分隔的路径列表；用 `-` 表示从 stdin 读
-0, --null               配合 --files-from，路径以 NUL 分隔
```

### 退出码

- `0` —— 所有输入都成功检测
- `1` —— 有输入读不出来（错误打到 stderr，其余输入继续检测）

## 工作原理

1. **BOM 检查** —— 文件以 UTF-8 / UTF-16 / UTF-32 BOM 开头就直接定编码（UTF-32 在 UTF-16 之前检查，因为前 2 字节相同），置信度为 `1`。
2. **ASCII 快速路径** —— 纯 ASCII 直接报 `utf-8`，置信度 `1`。
3. **chardetng 启发式** —— 取前 2 KB 喂给 chardetng，返回一个最佳猜测的编码名 + 布尔置信度（这个布尔值就是 `<confidence>` 列）。默认禁用 ISO-2022-JP（对任意输入更安全），允许 UTF-8。

样本有上限，大文件不会被整块读进内存。BOM 检测只需前 4 字节。

## 限制

- `<confidence>` 只是 1/0 标记 —— chardetng 不提供数值置信度，故没有 0–100 的分级。
- 不输出 JSON（刻意为之 —— TSV 才是 agent / 管道友好的格式）。
- 检测是启发式的，短样本可能误判；带 BOM 的文件一定准确。
- 目录模式下跳过符号链接。

## License

[Apache-2.0](LICENSE)。基于 Henri Sivonen 的 [chardetng](https://crates.io/crates/chardetng)（Apache-2.0 OR MIT）。
