export class ParseError extends Error {
  errors: { message: string; position: number; suggestion: string | null }[];

  constructor(
    errors: { message: string; position: number; suggestion: string | null }[]
  ) {
    const msg = errors.map((e) => e.message).join("; ");
    super(`Parse error: ${msg}`);
    this.name = "ParseError";
    this.errors = errors;
  }
}

export class RollError extends Error {
  constructor(message: string) {
    super(`Roll error: ${message}`);
    this.name = "RollError";
  }
}

export class DistributionError extends Error {
  constructor(message: string) {
    super(`Distribution error: ${message}`);
    this.name = "DistributionError";
  }
}
