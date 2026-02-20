#!/usr/bin/env node

const { execFileSync } = require("child_process");
const { join } = require("path");

const PLATFORMS = {
  "darwin-arm64": "@slipbridge/cli-darwin-arm64",
  "darwin-x64": "@slipbridge/cli-darwin-x64",
  "linux-x64": "@slipbridge/cli-linux-x64-gnu",
  "linux-arm64": "@slipbridge/cli-linux-arm64-gnu",
  "win32-x64": "@slipbridge/cli-win32-x64",
};

function getBinaryPath() {
  const key = `${process.platform}-${process.arch}`;
  const pkg = PLATFORMS[key];

  if (!pkg) {
    throw new Error(
      `slipbridge: unsupported platform ${key}. Supported: ${Object.keys(PLATFORMS).join(", ")}`
    );
  }

  try {
    const pkgDir = require.resolve(`${pkg}/package.json`);
    const binName = process.platform === "win32" ? "slipbridge.exe" : "slipbridge";
    return join(pkgDir, "..", binName);
  } catch {
    throw new Error(
      `slipbridge: could not find package ${pkg}. Make sure it was installed correctly.\n` +
        `Try reinstalling with: npm install slipbridge`
    );
  }
}

try {
  const bin = getBinaryPath();
  const result = execFileSync(bin, process.argv.slice(2), {
    stdio: "inherit",
    env: process.env,
  });
} catch (err) {
  if (err.status !== undefined) {
    process.exit(err.status);
  }
  console.error(err.message);
  process.exit(1);
}
