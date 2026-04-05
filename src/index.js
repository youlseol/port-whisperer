#!/usr/bin/env node

import { spawnSync } from "child_process";
import { existsSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const manifestPath = join(rootDir, "rust", "Cargo.toml");
const args = process.argv.slice(2);

if (!existsSync(manifestPath)) {
  console.error("Rust project not found at ./rust/Cargo.toml");
  process.exit(1);
}

const result = spawnSync(
  "cargo",
  ["run", "--manifest-path", manifestPath, "--bin", "ports", "--", ...args],
  {
    cwd: rootDir,
    stdio: "inherit",
    shell: process.platform === "win32",
  },
);

if (result.error) {
  console.error(`Failed to run cargo: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 1);
