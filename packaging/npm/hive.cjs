#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const { join } = require("node:path");

const filename = process.platform === "win32" ? "hive.exe" : "hive";
const binary = join(__dirname, "bin", process.platform, process.arch, filename);
const args = process.argv.slice(2);

// Replace the launcher when supported so Hive owns the terminal directly.
if (process.platform !== "win32" && typeof process.execve === "function") {
  try {
    process.execve(binary, [binary, ...args], process.env);
  } catch (error) {
    console.error(`hive: ${error.message}`);
    process.exit(1);
  }
}

const result = spawnSync(binary, args, { stdio: "inherit" });
if (result.error) {
  console.error(`hive: ${result.error.message}`);
}
process.exitCode = result.status ?? 1;
