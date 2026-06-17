export interface ParseError {
  message: string;
  position: number;
  suggestion: string | null;
}

export interface Expression {
  [key: string]: unknown;
}

export interface ParseResultSuccess {
  success: true;
  expression: Expression;
}

export interface ParseResultFailure {
  success: false;
  errors: ParseError[];
}

export type ParseResult = ParseResultSuccess | ParseResultFailure;

export interface DieEntry {
  value: number;
  kept: boolean;
  chain: number[] | null;
  kind: string | null;
  operator: string | null;
}

export interface RollResult {
  value: number;
  dice: DieEntry[];
}

export interface RollOptions {
  seed?: number;
}

export interface Stats {
  min: number;
  max: number;
  mean: number;
  stddev: number;
  variance: number;
}

export interface DistributionResult {
  distribution: Record<string, number>;
  total: number;
  stats: Stats;
}

export interface DistributionOptions {
  trials?: number;
}
