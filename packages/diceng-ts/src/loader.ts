import { join } from "path";
import { existsSync } from "fs";

const PLATFORM_MAP: Record<string, Record<string, string>> = {
  linux: {
    x64: "linux-x86_64",
    arm64: "linux-aarch64",
  },
  darwin: {
    x64: "macos-x86_64",
    arm64: "macos-aarch64",
  },
  win32: {
    x64: "windows-x86_64",
  },
};

function getPlatformDir(): string {
  const platform = process.platform;
  const arch = process.arch;
  const map = PLATFORM_MAP[platform];
  if (!map) {
    throw new Error(`Unsupported platform: ${platform}`);
  }
  const dir = map[arch];
  if (!dir) {
    throw new Error(`Unsupported architecture: ${arch} on ${platform}`);
  }
  return dir;
}

function getSuffix(): string {
  switch (process.platform) {
    case "darwin":
      return "dylib";
    case "win32":
      return "dll";
    default:
      return "so";
  }
}

export function findBinary(): string {
  const envPath = process.env.DICENG_PATH;
  if (envPath && existsSync(envPath)) {
    return envPath;
  }

  const platformDir = getPlatformDir();
  const suffix = getSuffix();
  const libName = `libdiceng.${suffix}`;

  const candidates = [
    join(__dirname, "..", "..", "..", "dist", platformDir, libName),
    join(__dirname, "..", "dist", platformDir, libName),
    join(process.cwd(), "dist", platformDir, libName),
  ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }

  throw new Error(
    `diceng native library not found. Searched:\n` +
      candidates.map((c) => `  ${c}`).join("\n") +
      `\n\nSet DICENG_PATH env var or run: make build-so`
  );
}
