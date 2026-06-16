# diceng

TypeScript wrapper for [diceng](https://github.com/Syrup/diceng), a dice expression parser and roller written in Rust.

> **This is experimental.** The `bun-ffi-support` branch is not stable. APIs
> may change, break, or disappear without notice. Bugs might get fixed next
> week, or next year, or never. Use at your own risk.

## Requirements

- [Bun](https://bun.sh) v1.0+
- Built `libdiceng.so` (or `.dylib`/`.dll` on other platforms)

## Install

First, build the native library:

```bash
# from the repo root
make build-so
```

Then use the package locally:

```bash
cd packages/diceng-ts
bun install
```

## Usage

```ts
import { parse, roll, rollDice, distribution } from "diceng";

// rollDice: parse + roll in one call
const result = rollDice("4d6kh3");
console.log(result.value); // e.g. 15
console.log(result.dice);  // [{value: 6, kept: true, ...}, ...]

// parse then roll (reusable expression)
const parsed = parse("3d6!");
if (!parsed.success) {
  throw new Error(parsed.errors.map(e => e.message).join("; "));
}
const r = roll(parsed.expression, { seed: 42 });

// probability distribution
const dist = distribution(parsed.expression, { trials: 10_000 });
console.log(dist.stats.mean);
```

## API

### `parse(input: string): ParseResult`

Parses a dice expression. Returns `{ success: true, expression }` or `{ success: false, errors }`.

### `roll(expression: Expression, options?: RollOptions): RollResult`

Rolls a parsed expression. Returns `{ value, dice[] }`.

Options:
- `seed?: number` - deterministic seed. Omit for random.

### `rollDice(input: string, options?: RollOptions): RollResult`

Convenience. Calls `parse()` then `roll()` in one step. Throws on parse error.

### `distribution(expression: Expression, options?: DistributionOptions): DistributionResult`

Computes probability distribution. Returns `{ distribution, total, stats }`.

Options:
- `trials?: number` - Monte Carlo trials (default 10,000, used when exact computation fails)

## DieEntry

Each item in `dice[]` has:

| Field | Type | Meaning |
|-------|------|---------|
| `value` | `number` | Final die value |
| `kept` | `boolean` | `true` if counts toward result |
| `chain` | `number[] \| null` | Roll history for explode/reroll |
| `kind` | `string \| null` | "Explode", "Reroll", "MinCap", etc. |
| `operator` | `string \| null` | Arithmetic operator between groups |

## Binary location

The loader checks these paths in order:

1. `DICENG_PATH` environment variable
2. `dist/<platform>/libdiceng.*` relative to the repo root
3. `dist/<platform>/libdiceng.*` relative to the package

## License

MIT
