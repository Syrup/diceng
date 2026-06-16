# diceng

![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-blue)
![Version](https://img.shields.io/badge/version-1.0.0-green)
![CI](https://github.com/Syrup/diceng/actions/workflows/ci.yml/badge.svg)
![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows%20%7C%20android-lightgrey)
![GitHub Stars](https://img.shields.io/github/stars/Syrup/diceng?style=social)

> Fast dice expression parser, roller, and probability analyzer written in Rust.

## Features

- **Parse** standard dice notation — `d6`, `3d6`, `d20`, `d%`, `dF`, `dF.2`, custom faces
- **Roll20/Foundry compatible** — `!` explode, `kh`/`kl`/`dh`/`dl`, `!!` compound, `cs` count
- **Roll** with true random or seeded (deterministic, reproducible) RNG
- **Exact probability** computation via convolution + dynamic programming (Eisenstat's algorithm)
- **Monte Carlo** simulation fallback with automatic convergence detection
- **Rich modifiers** — keep/drop (highest/lowest/middle), explode, reroll, compound, emphasis, min/max cap
- **Arithmetic expressions** with proper operator precedence (`3d6 + 2d4 * 2`)
- **Dice sets** with reducers — sum, min, max, average, median
- **Count thresholds** — `4d6 c>=5` or `4d6t4` counts dice meeting a condition
- **Unicode support** — `×`, `⋅`, `÷`, `≥`, `≤`, `≠` operators
- **CLI** with text and JSON output formats
- **Multi-platform** — Linux, macOS, Windows, Android (Termux)

## Installation

### From source

```bash
git clone https://github.com/Syrup/diceng.git
cd diceng
make release    # build + strip
make install    # install to ~/.cargo/bin
```

### From crates.io

```bash
cargo install diceng
```

### As a library

```toml
[dependencies]
diceng = "0.1"
```

## Quick Start

```bash
# Roll 4d6 keep highest 3
diceng roll 4d6k3

# Roll with verbose output
diceng roll 4d6k3 -v

# Roll with deterministic seed
diceng roll 2d6 --seed 42

# Get probability distribution
diceng stats 2d6

# Validate an expression
diceng check "3d6e5 + 2d4"
```

## Usage

### `roll` — Roll a dice expression

```bash
diceng roll <expression> [options]
```

| Option | Short | Description |
|--------|-------|-------------|
| `--verbose` | `-v` | Show detailed roll breakdown |
| `--format <fmt>` | `-f` | Output format: `text` (default) or `json` |
| `--seed <n>` | | Deterministic seed for reproducible rolls |

**Examples:**

```bash
diceng roll 4d6k3 -v
# 4d6k3 = 14
#
# [5]✓  [5]✓  [4]✓  [2]✗
#
# Kept: 5 + 5 + 4 = 14
# Drop: 2 = 2

diceng roll 2d6 -f json
# {
#   "expression": "2d6",
#   "seed": null,
#   "result": 7,
#   "dice": [3, 4]
# }
```

### `stats` — Compute probability distribution

```bash
diceng stats <expression> [options]
```

| Option | Short | Description |
|--------|-------|-------------|
| `--format <fmt>` | `-f` | Output format: `text` (default) or `json` |
| `--trials <n>` | | Monte Carlo trials if exact fails (default: 100000) |
| `--precision <n>` | | Decimal places for probabilities (default: 4) |

**Example:**

```bash
diceng stats 2d6
# Expression: 2d6
# Method: Exact
#
# Min: 2
# Max: 12
# Mean: 7.0000
# Stddev: 2.4152
#
# Distribution:
#    Value       Prob   At Least    At Most      Count
# ----------------------------------------------------
#        2     0.0278     1.0000     0.0278          1
#        3     0.0556     0.9722     0.0833          2
#      ...
#       12     0.0278     0.0278     1.0000          1
```

### `check` — Validate a dice expression

```bash
diceng check <expression>
```

**Example:**

```bash
diceng check "4d6k3"
# ✓ Valid expression: 4d6k3

diceng check "invalid"
# ✗ Invalid expression: invalid
#   - Position 0: Expected number or 'd', got 'i'
```

## Supported Dice Notation

### Dice Types

| Type | Syntax | Example | Description |
|------|--------|---------|-------------|
| Standard | `NdS` | `3d6`, `d20` | N dice with S sides |
| Percent | `Nd%` | `d%`, `2d%` | 1-100 range |
| Fate/Fudge | `NdF` | `4dF` | Faces: -1, 0, +1 |
| Variable Fudge | `NdF.N` | `dF.2` | Faces: -2, -1, 0, 0, +1, +2 |
| Custom | `Nd{faces}` | `d{1,2,3}` | Arbitrary face values |

### Modifiers (Functors)

| Modifier | Standard | diceng | Long form | Example |
|----------|----------|----------|-----------|---------|
| Explode | `!` | `e` | `explode` | `3d6!` or `3d6e5` |
| Compound | `!!` | `ce` | `compound` | `3d6!!` or `3d6ce6` |
| Reroll | | `r` | `reroll` | `3d6r1` — reroll 1s |
| Reroll once | `ro` | | | `2d6ro1` — reroll 1s once |
| Emphasis | | | `emphasis` | `2d6 emphasis` — furthest from center |
| Min cap | `mi` | | | `4d6mi2` — minimum 2 per die |
| Max cap | `ma` | | | `4d6ma5` — maximum 5 per die |

**Explode conditions:** `3d6!` (on max), `3d6!>=5`, `3d6!>5`, `3d6!5`

**Limits:** `once`, `twice`, `thrice`, `N times`

### Filters

| Filter | Standard | diceng | Directions | Example |
|--------|----------|----------|------------|---------|
| Keep highest | `kh` | `k` | highest (default) | `4d6kh3` or `4d6k3` |
| Keep lowest | `kl` | | lowest | `4d6kl1` |
| Drop highest | `dh` | | highest | `4d6dh1` |
| Drop lowest | `dl` | `d` | lowest (default) | `4d6dl1` or `4d6d1` |
| Keep middle | | | middle | `5d6 keep middle 3` |
| Sort ascending | `sa` | | | `4d6sa` |
| Sort descending | `sd` | | | `4d6sd` |

### Count Thresholds

Count dice meeting a condition:

```
4d6 c>=5       — count dice >= 5
4d6 c6         — count dice equal to 6
4d6 c>=3 c<=5  — count dice between 3 and 5
4d6 cs>=4      — count successes (same as c>=4)
4d6 t4         — target number 4 (defaults to >=4)
```

### Reducers (for Dice Sets)

| Reducer | Example |
|---------|---------|
| sum | `(2d6, 3d6) sum` |
| min | `(2d6, 3d6) min` |
| max | `[d6, d8] max` |
| average | `[d6, d8] average` |
| median | `[d6, d8] median` |

### Arithmetic

```
3d6 + 4          — addition
2d6 * 3          — multiplication
(3d6 + 2) * 2    — with parentheses
-d6 + 10         — unary minus
```

**Operators:** `+`, `-`, `*`, `/` (and Unicode `×`, `⋅`, `÷`)

## Architecture

```
                    ┌─────────────┐
                    │   Input     │
                    │  "4d6k3"    │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │    Lexer    │
                    │  lexer.rs   │
                    └──────┬──────┘
                           │ Token stream
                    ┌──────▼──────┐
                    │ Pratt Parser│
                    │  pratt.rs   │
                    └──────┬──────┘
                           │ AST (Expression)
                ┌──────────┼──────────┐
                │                     │
         ┌──────▼──────┐      ┌──────▼──────┐
         │   Roller    │      │    Stats    │
         │  eval.rs    │      │ exact.rs /  │
         │ + rng.rs    │      │ monte_carlo │
         └──────┬──────┘      └──────┬──────┘
                │                     │
         ┌──────▼──────┐      ┌──────▼──────┐
         │ RollResult  │      │Probabilities│
         │  (tree)     │      │   Result    │
         └──────┬──────┘      └──────┬──────┘
                │                     │
         ┌──────▼──────┐      ┌──────▼──────┐
         │  Display    │      │    Stats    │
         │ (terminal)  │      │  (table)    │
         └─────────────┘      └─────────────┘
```

### Modules

| Module | Purpose |
|--------|---------|
| `parser/` | Lexer (788 LOC) + Pratt parser (612 LOC) → AST |
| `roller/` | Dice roller engine (798 LOC) + RNG trait + implementations |
| `stats/` | Exact probability via DP (757 LOC) + Monte Carlo simulation |
| `display.rs` | Terminal verbose output with colors |
| `types.rs` | Shared enums and structs |

## API Reference

### Convenience Functions

```rust
use diceng::*;

// Parse a dice expression
let result = parse("4d6k3");
assert!(result.success());
let expr = result.expression().unwrap();

// Roll (non-deterministic)
let roll = roll(expr);
println!("{}", roll.value());

// Roll (deterministic, seeded)
let roll = roll_seeded(expr, 42);

// Exact probability distribution
let dist = exact_distribution(expr);

// Monte Carlo fallback
let dist = monte_carlo_distribution(expr, 100_000);

// Auto-select (exact first, then Monte Carlo)
let dist = compute_distribution(expr, 100_000);
```

### Key Types

```rust
// RNG trait — implement custom randomness source
pub trait DiceRng {
    fn roll(&mut self, sides: u32) -> u32;
}

// Built-in RNG implementations
pub struct RandomRng;      // true random (rand crate)
pub struct LehmerRng;      // deterministic seeded PRNG

// Probability result
pub struct ProbabilitiesResult {
    pub distribution: HashMap<i64, u64>,
    pub total: u64,
}

// Stats summary
pub struct Stats {
    pub min: i64,
    pub max: i64,
    pub mean: f64,
    pub stddev: f64,
    pub variance: f64,
    pub distribution: Vec<(i64, f64)>,
}
```

## Multi-platform Build

diceng uses a Makefile for cross-compilation:

```bash
# Default: Linux x86_64
make build

# Android
make build PLATFORM=android                           # aarch64 (default)
make build PLATFORM=android ANDROID_ARCH=armv7        # ARM32
make build PLATFORM=android ANDROID_ARCH=x86_64       # Emulator 64-bit

# Linux
make build PLATFORM=linux                             # x86_64
make build PLATFORM=linux LINUX_ARCH=aarch64          # ARM64
make build PLATFORM=linux LINUX_ARCH=x86_64-musl      # Static binary

# macOS
make build PLATFORM=macos                             # Apple Silicon
make build PLATFORM=macos MACOS_ARCH=x86_64           # Intel

# Windows
make build PLATFORM=windows                           # x86_64 MSVC

# Build all platforms
make build-all

# Other targets
make release    # build + strip
make check      # fmt + clippy + test
make clean      # clean artifacts
```

### Android Setup

```bash
make build PLATFORM=android
# cargo-ndk not found.
#
# Install now? [y/N] y
# Installing cargo-ndk...
#
# Android NDK r27c not found.
#
# Install now? [y/N] y
# Downloading Android NDK r27c...
```

## Algorithm

### Lehmer/Park-Miller PRNG

The seeded RNG uses the MINSTD variant (Park, Miller & Stockmeyer, 1993):

```
X_{n+1} = (48271 * X_n) mod (2^31 - 1)
```

- **Period:** 2,147,483,646 (~2.1 billion unique values)
- **Deterministic:** Same seed always produces the same sequence
- **Overflow-safe:** Uses `u64` widening for multiplication

### Eisenstat's DP Algorithm

Exact probability for keep/drop filters uses dynamic programming instead of brute-force enumeration:

| Approach | Complexity |
|----------|-----------|
| Brute force | O(sides^count) |
| Eisenstat's DP | O(count^3 * sides^2 * drop) |

For `10d6 keep 3`, brute force = 6^10 ≈ 60M iterations. DP = ~10K operations.

### Memory Safety

- 1GB allocator cap via the `cap` crate prevents runaway memory usage
- `MAX_DICE_COUNT = 10,000` limits dice pool size
- Overflow-checked arithmetic in probability calculations

## Contributing

1. Fork the repository
2. Create a feature branch
3. Run `make check` (fmt + clippy + test)
4. Submit a pull request

```bash
make check
# cargo fmt --check
# cargo clippy -- -D warnings
# cargo test
# 78 tests passed
```

## License

MIT — see [LICENSE](LICENSE) for details.
