import { ffi_parse, ffi_roll, ffi_roll_dice, ffi_compute_distribution } from "./ffi";
import { ParseErrorClass, RollError, DistributionError } from "./errors";
import type {
  ParseResult,
  Expression,
  RollResult,
  RollOptions,
  DistributionResult,
  DistributionOptions,
} from "./types";

export function parse(input: string): ParseResult {
  const raw = ffi_parse(input) as {
    success: boolean;
    expression?: Expression;
    errors?: { message: string; position: number; suggestion: string | null }[];
  };

  if (raw.success) {
    return { success: true, expression: raw.expression! };
  }
  return { success: false, errors: raw.errors! };
}

export function roll(expression: Expression, options?: RollOptions): RollResult {
  const seed = options?.seed ?? -1;
  const exprJson = JSON.stringify(expression);
  const raw = ffi_roll(exprJson, seed) as
    | { value: number; dice: { value: number; kept: boolean; chain: number[] | null; kind: string | null; operator: string | null }[] }
    | { error: string };

  if ("error" in raw) {
    throw new RollError(raw.error);
  }

  return {
    value: raw.value,
    dice: raw.dice.map((d) => ({
      value: d.value,
      kept: d.kept,
      chain: d.chain,
      kind: d.kind,
      operator: d.operator,
    })),
  };
}

export function rollDice(input: string, options?: RollOptions): RollResult {
  const seed = options?.seed ?? -1;
  const raw = ffi_roll_dice(input, seed) as
    | { value: number; dice: { value: number; kept: boolean; chain: number[] | null; kind: string | null; operator: string | null }[] }
    | { error: string };

  if ("error" in raw) {
    throw new RollError(raw.error);
  }

  return {
    value: raw.value,
    dice: raw.dice.map((d) => ({
      value: d.value,
      kept: d.kept,
      chain: d.chain,
      kind: d.kind,
      operator: d.operator,
    })),
  };
}

export function distribution(
  expression: Expression,
  options?: DistributionOptions
): DistributionResult {
  const trials = options?.trials ?? 10_000;
  const exprJson = JSON.stringify(expression);
  const raw = ffi_compute_distribution(exprJson, trials) as
    | { distribution: Record<string, number>; total: number; stats: { min: number; max: number; mean: number; stddev: number; variance: number } }
    | { error: string };

  if ("error" in raw) {
    throw new DistributionError(raw.error);
  }

  return {
    distribution: raw.distribution,
    total: raw.total,
    stats: raw.stats,
  };
}
