import { findBinary } from "../src/loader";

try {
  const path = findBinary();
  console.log(`diceng: found native library at ${path}`);
} catch {
  console.warn(
    "diceng: native library not found. Build it with:\n" +
      "  cd ../.. && make build-so\n" +
      "\n" +
      "Or set DICENG_PATH environment variable to the library path."
  );
}
