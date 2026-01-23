# lucidshark-duplo

A fast code duplication detection tool written in Rust. Finds duplicate code blocks across multiple files to help identify candidates for refactoring.

## Features

- **Fast parallel processing** - Uses all available CPU cores via Rayon
- **Multiple output formats** - Console (human-readable), JSON, and XML
- **Language-aware** - Strips comments and filters noise (imports, preprocessor directives) for accurate detection
- **Configurable thresholds** - Set minimum block size and character limits
- **CI/CD friendly** - Exit code 1 when duplicates found, 0 when clean

## Supported Languages

| Language | Extensions | Comment Handling |
|----------|------------|------------------|
| C/C++ | `.c`, `.cpp`, `.cxx`, `.h`, `.hpp` | `//`, `/* */`, preprocessor filtering |
| C# | `.cs` | `//`, `/* */`, preprocessor filtering |
| Java | `.java` | `//`, `/* */`, `/** */`, import filtering |
| JavaScript/TypeScript | `.js`, `.ts`, `.jsx`, `.tsx` | `//`, `/* */`, JSDoc, import/require filtering |
| Python | `.py` | `#`, `"""` docstrings, import filtering |
| Rust | `.rs` | `//`, `/* */` (nested), `use` filtering |
| HTML | `.html`, `.htm` | `<!-- -->` |
| CSS | `.css` | `/* */`, `@import` filtering |
| Visual Basic | `.vb` | `'` comments, `Imports` filtering |
| Erlang | `.erl`, `.hrl` | `%` comments, `-module` filtering |

## Installation

### From source

```bash
git clone https://github.com/lucidshark-code/lucidshark-duplo.git
cd lucidshark-duplo
cargo build --release
```

The binary will be at `target/release/lucidshark-duplo`.

## Usage

```bash
lucidshark-duplo [OPTIONS] <FILE_LIST>
```

The `FILE_LIST` is a text file containing paths to analyze, one per line.

### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--min-lines <N>` | `-m` | Minimum duplicate block size in lines | 4 |
| `--min-chars <N>` | `-c` | Minimum characters per line to consider | 3 |
| `--ignore-preprocessor` | `-i` | Ignore preprocessor directives | false |
| `--ignore-same-filename` | `-s` | Ignore duplicates in files with same name | false |
| `--json` | | Output in JSON format | |
| `--xml` | | Output in XML format | |
| `--help` | `-h` | Print help | |
| `--version` | `-V` | Print version | |

### Examples

Create a file list:
```bash
find src -name "*.rs" > files.txt
```

Run with default settings:
```bash
lucidshark-duplo files.txt
```

Output as JSON with minimum 10-line blocks:
```bash
lucidshark-duplo --json -m 10 files.txt
```

Ignore preprocessor directives in C code:
```bash
lucidshark-duplo -i c_files.txt
```

## Output Formats

### Console (default)

```
Configuration:
  Minimum block size: 4 lines
  Minimum characters: 3
  Ignore preprocessor: false

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
  "duplicates": [
    {
      "line_count": 5,
      "file1": {
        "path": "src/foo.rs",
        "start_line": 10,
        "end_line": 14
      },
      "file2": {
        "path": "src/bar.rs",
        "start_line": 25,
        "end_line": 29
      },
      "lines": ["let x = compute_value();", "..."]
    }
  ],
  "summary": {
    "files_analyzed": 42,
    "total_lines": 8521,
    "duplicate_blocks": 7,
    "duplicate_lines": 89,
    "duplication_percent": 1.04
  }
}
```

### XML

```xml
<?xml version="1.0" encoding="UTF-8"?>
<duplo>
  <set LineCount="5">
    <block SourceFile="src/foo.rs" StartLineNumber="10" EndLineNumber="14"/>
    <block SourceFile="src/bar.rs" StartLineNumber="25" EndLineNumber="29"/>
  </set>
  <summary Files="42" Lines="8521" Duplicates="7" DuplicateLines="89" Percent="1.04"/>
</duplo>
```

## How It Works

1. **Load files** - Read each file and identify language by extension
2. **Preprocess** - Strip comments and filter noise based on language rules
3. **Hash lines** - Compute FNV-1a hash for each normalized line
4. **Build index** - Create hash-to-files lookup for candidate matching
5. **Compare pairs** - For files sharing hashes, use matrix algorithm to find longest common subsequences
6. **Extract blocks** - Identify contiguous duplicate regions meeting minimum size threshold
7. **Report** - Output results in requested format

The algorithm is based on the approach from [Duplo](https://github.com/dlidstrom/Duplo), reimplemented in Rust with parallel processing.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | No duplicates found |
| 1 | Duplicates found |
| 2 | Error (missing files, invalid arguments, etc.) |

## Running Tests

```bash
cargo test
```

This runs both unit tests (72) and integration tests (28) covering CLI behavior, detection accuracy, and output format validation.

## Acknowledgments

This project is a Rust reimplementation of [Duplo](https://github.com/dlidstrom/Duplo) by Daniel Lidström. The core algorithm for detecting duplicate code blocks using matrix-based longest common subsequence matching is based on Duplo's approach.

## License

Apache 2.0 - See [LICENSE](LICENSE) for details.
