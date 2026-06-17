import { parse, roll, rollDice } from "../src/";

// rollDice() - parse + roll, throws on error
console.log(rollDice("4d6kh3"));

// roll() - need expression from parse()
const result = parse("3d6!");
if (!result.success) {
  throw new Error("Parse error: " + result.errors.map((e) => e.message).join("; "));
}
console.log(roll(result.expression, { seed: 10 }));
