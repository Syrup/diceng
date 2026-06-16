export { parse } from "./api";
export { roll } from "./api";
export { rollDice } from "./api";
export { distribution } from "./api";

export type {
  ParseResult,
  ParseResultSuccess,
  ParseResultFailure,
  ParseError,
  Expression,
  DieEntry,
  RollResult,
  RollOptions,
  Stats,
  DistributionResult,
  DistributionOptions,
} from "./types";

export { ParseError as ParseErrorClass, RollError, DistributionError } from "./errors";
