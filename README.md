# lucidshark-duplo

A fast, feature-rich code duplication detection tool written in Rust. Finds duplicate code blocks across multiple files to help identify candidates for refactoring.

## Features

- **Git integration** - Analyze tracked files automatically, or focus only on changed files for PR reviews
- **Incremental caching** - Cache processed files for faster subsequent runs
- **Baseline comparison** - Track known duplicates and only report new ones in CI/CD
- **Fast parallel processing** - Uses all available CPU cores
- **Multiple output formats** - Console, JSON, and XML
- **Language-aware** - Smart filtering of comments, imports, docstrings, and boilerplate
- **Configurable thresholds** - Set minimum block size and character limits

## Supported Languages

| Language | Extensions | Filtering |
|----------|------------|-----------|
| C/C++ | `.c`, `.cpp`, `.cxx`, `.h`, `.hpp` | Comments, preprocessor directives |
| C# | `.cs` | Comments, preprocessor directives |
| Java | `.java` | Comments, JavaDoc, imports, annotations, method signatures |
| JavaScript/TypeScript | `.js`, `.ts`, `.jsx`, `.tsx` | Comments, JSDoc, imports, decorators, function signatures |
| Python | `.py` | Comments, docstrings, imports, decorators, function signatures |
| Rust | `.rs` | Comments (nested), `use` statements, attributes, function signatures |
| HTML | `.html`, `.htm` | HTML comments |
| CSS | `.css` | Comments, `@import` statements |
| Visual Basic | `.vb` | Comments, `Imports` statements |
| Erlang | `.erl`, `.hrl` | Comments, `-module` declarations |

## Installation

```bash
git clone https://github.com/lucidshark-code/lucidshark-duplo.git
cd lucidshark-duplo
cargo build --release
```

The binary will be at `target/release/lucidshark-duplo`.

## Usage

### Basic Usage

```bash
# From a file list
lucidshark-duplo files.txt

# From git repository (all tracked files)
lucidshark-duplo --git

# Only files changed vs main branch
lucidshark-duplo --git --changed-only
```

### Options

| Option | Description |
|--------|-------------|
| `--git` | Discover files from git (tracked files) |
| `--changed-only` | Only analyze files changed vs base branch |
| `--base-branch <BRANCH>` | Base branch for comparison (auto-detects main/master) |
| `--cache` | Enable incremental caching |
| `--cache-dir <DIR>` | Cache directory (default: `.duplo-cache`) |
| `--clear-cache` | Clear cache before running |
| `--baseline <FILE>` | Compare against baseline, report only NEW duplicates |
| `--save-baseline <FILE>` | Save results as baseline for future comparison |
| `-m, --min-lines <N>` | Minimum duplicate block size (default: 4) |
| `-c, --min-chars <N>` | Minimum characters per line (default: 3) |
| `-s, --ignore-same-filename` | Ignore duplicates in files with same name |
| `--json` | Output in JSON format |
| `--xml` | Output in XML format |

### Examples

**Find duplicates in a git repository:**
```bash
lucidshark-duplo --git
```

**PR workflow - only check changed files:**
```bash
lucidshark-duplo --git --changed-only --base-branch main
```

**CI/CD with baseline - fail only on NEW duplicates:**
```bash
# First run: establish baseline
lucidshark-duplo --git --save-baseline baseline.json

# Subsequent runs: only fail on new duplicates
lucidshark-duplo --git --baseline baseline.json
```

**Fast repeated runs with caching:**
```bash
lucidshark-duplo --git --cache
```

**JSON output with minimum 10-line blocks:**
```bash
lucidshark-duplo --git --json -m 10
```

## Output Formats

### Console (default)

```
Configuration:
  Minimum block size: 4 lines
  Minimum characters: 3

Found duplicate block (5 lines):
  src/foo.rs (10-14) <-> src/bar.rs (25-29)

    let x = compute_value();
    let y = transform(x);
    let z = finalize(y);
    return z;

Summary:
  Files analyzed: 42
  Total lines: 8,521
  Duplicate blocks: 7
  Duplicate lines: 89
  Duplication: 1.04%
```

### JSON

```json
{
  "duplicates": [...],
  "summary": {
    "files_analyzed": 42,
    "total_lines": 8521,
    "duplicate_blocks": 7,
    "duplicate_lines": 89,
    "duplication_percent": 1.04
  }
}
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | No duplicates found (or no NEW duplicates with baseline) |
| 1 | Duplicates found |
| 2 | Error |

## Running Tests

```bash
cargo test
```

## License

Apache 2.0 - See [LICENSE](LICENSE) for details.
