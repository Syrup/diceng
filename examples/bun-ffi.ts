/**
 * diceng — Bun FFI Example
 *
 * Usage:
 *   bun run examples/bun-ffi.ts
 *
 * Prerequisites:
 *   1. Build the shared library: make build-so
 *   2. Ensure libdiceng.so is in dist/linux-x86_64/
 */

import { dlopen, FFIType, suffix, ptr, CString } from "bun:ffi";
import { join } from "path";

const LIB_PATH = join(__dirname, "..", "dist", `linux-x86_64`, `libdiceng.${suffix}`);

const {
  symbols: { diceng_parse, diceng_roll, diceng_compute_distribution, diceng_free },
} = dlopen(LIB_PATH, {
  diceng_parse: {
    args: [FFIType.cstring],
    returns: FFIType.ptr,
  },
  diceng_roll: {
    args: [FFIType.cstring, FFIType.i64],
    returns: FFIType.ptr,
  },
  diceng_compute_distribution: {
    args: [FFIType.cstring, FFIType.u64],
    returns: FFIType.ptr,
  },
  diceng_free: {
    args: [FFIType.ptr],
    returns: FFIType.void,
  },
});

function parse(expression: string) {
  const input = Buffer.from(expression + "\0", "utf8");
  const resultPtr = diceng_parse(ptr(input));
  if (!resultPtr) throw new Error("diceng_parse returned null");
  const result = new CString(resultPtr);
  diceng_free(resultPtr);
  return JSON.parse(result);
}

function roll(expressionJson: string, seed = -1) {
  const input = Buffer.from(expressionJson + "\0", "utf8");
  const resultPtr = diceng_roll(ptr(input), BigInt(seed));
  if (!resultPtr) throw new Error("diceng_roll returned null");
  const result = new CString(resultPtr);
  diceng_free(resultPtr);
  return JSON.parse(result);
}

function computeDistribution(expressionJson: string, trials = 10_000) {
  const input = Buffer.from(expressionJson + "\0", "utf8");
  const resultPtr = diceng_compute_distribution(ptr(input), BigInt(trials));
  if (!resultPtr) throw new Error("diceng_compute_distribution returned null");
  const result = new CString(resultPtr);
  diceng_free(resultPtr);
  return JSON.parse(result);
}

// ── Demo ──────────────────────────────────────────

console.log("=== diceng Bun FFI Demo ===\n");

// 1. Parse a dice expression
console.log('parse("3d6+4"):');
const parsed = parse("3d6+4");
console.log(JSON.stringify(parsed, null, 2));
console.log();

// 2. Roll with seed for deterministic result
if (parsed.success) {
  console.log('roll("3d6+4", seed=42):');
  const rollResult = roll(JSON.stringify(parsed.expression), 42);
  console.log(rollResult);
  console.log();

  // 3. Roll random
  console.log('roll("3d6+4", random):');
  const randomRoll = roll(JSON.stringify(parsed.expression));
  console.log(randomRoll);
  console.log();

  // 4. Compute probability distribution
  console.log('computeDistribution("3d6+4", 10000):');
  const dist = computeDistribution(JSON.stringify(parsed.expression), 10_000);
  console.log(`  total outcomes: ${dist.total}`);
  console.log(`  range: ${dist.stats.min} - ${dist.stats.max}`);
  console.log(`  mean: ${dist.stats.mean.toFixed(2)}`);
  console.log(`  stddev: ${dist.stats.stddev.toFixed(2)}`);
}

// 5. Parse error handling
console.log('\nparse("invalid!!!"):');
const invalid = parse("invalid!!!");
console.log(JSON.stringify(invalid, null, 2));
