import { describe, test, expect } from "bun:test";
import { parse, roll, rollDice, distribution } from "../src/index";

describe("diceng", () => {
  describe("parse", () => {
    test("parses valid expression", () => {
      const result = parse("3d6+4");
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.expression).toBeDefined();
      }
    });

    test("returns errors for invalid expression", () => {
      const result = parse("invalid!!!");
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.errors.length).toBeGreaterThan(0);
        expect(result.errors[0].message).toBeDefined();
      }
    });

    test("parses dice pool", () => {
      const result = parse("{2d6, 1d8}");
      expect(result.success).toBe(true);
    });
  });

  describe("roll", () => {
    test("rolls parsed expression", () => {
      const parsed = parse("3d6+4");
      expect(parsed.success).toBe(true);
      if (!parsed.success) return;

      const result = roll(parsed.expression, { seed: 42 });
      expect(typeof result.value).toBe("number");
      expect(result.value).toBeGreaterThanOrEqual(7); // 3+4
      expect(result.value).toBeLessThanOrEqual(22); // 18+4
    });

    test("deterministic with same seed", () => {
      const parsed = parse("4d6kh3");
      expect(parsed.success).toBe(true);
      if (!parsed.success) return;

      const r1 = roll(parsed.expression, { seed: 42 });
      const r2 = roll(parsed.expression, { seed: 42 });
      expect(r1.value).toBe(r2.value);
    });

    test("returns verbose dice data", () => {
      const parsed = parse("4d6kh3");
      expect(parsed.success).toBe(true);
      if (!parsed.success) return;

      const result = roll(parsed.expression, { seed: 42 });
      expect(result.dice.length).toBeGreaterThan(0);
      expect(result.dice[0]).toHaveProperty("value");
      expect(result.dice[0]).toHaveProperty("kept");
    });
  });

  describe("rollDice", () => {
    test("parse + roll in one call", () => {
      const result = rollDice("3d6+4", { seed: 42 });
      expect(typeof result.value).toBe("number");
      expect(result.value).toBeGreaterThanOrEqual(7);
    });

    test("returns verbose dice data", () => {
      const result = rollDice("4d6kh3", { seed: 42 });
      expect(result.dice.length).toBeGreaterThan(0);
      expect(result.dice[0]).toHaveProperty("value");
      expect(result.dice[0]).toHaveProperty("kept");
    });

    test("throws on invalid expression", () => {
      expect(() => rollDice("invalid!!!")).toThrow();
    });
  });

  describe("distribution", () => {
    test("computes exact distribution for 2d6", () => {
      const parsed = parse("2d6");
      expect(parsed.success).toBe(true);
      if (!parsed.success) return;

      const dist = distribution(parsed.expression);
      expect(dist.total).toBe(36);
      expect(dist.stats.min).toBe(2);
      expect(dist.stats.max).toBe(12);
    });

    test("computes distribution via rollDice shortcut", () => {
      const parsed = parse("d6");
      expect(parsed.success).toBe(true);
      if (!parsed.success) return;

      const dist = distribution(parsed.expression, { trials: 1000 });
      expect(dist.total).toBe(6);
      expect(dist.stats.mean).toBeCloseTo(3.5, 0);
    });
  });
});
