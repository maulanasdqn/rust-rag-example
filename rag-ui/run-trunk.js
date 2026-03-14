#!/usr/bin/env node
"use strict";
/**
 * Launcher for Trunk on Windows. Prepends this directory to PATH so Trunk's
 * built-in hooks that call "sh" find our sh.cmd wrapper (no real sh on Windows).
 */
const path = require("path");
const { spawn } = require("child_process");

const dir = __dirname;
const sep = process.platform === "win32" ? ";" : ":";
const env = { ...process.env, PATH: dir + sep + process.env.PATH };

const trunk = spawn("trunk", ["serve", "--port", "8081"], {
  env,
  stdio: "inherit",
  shell: true,
  cwd: dir,
});

trunk.on("exit", (code) => process.exit(code ?? 0));
