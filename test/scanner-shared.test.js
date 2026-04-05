import test from "node:test";
import assert from "node:assert/strict";
import { detectFrameworkFromName, isDevProcess, summarizeCommand } from "../src/scanner-shared.js";

test("detectFrameworkFromName handles Windows executable suffixes", () => {
  assert.equal(detectFrameworkFromName("node.exe"), "Node.js");
  assert.equal(detectFrameworkFromName("python.exe"), "Python");
});

test("isDevProcess recognizes Windows runtime process names", () => {
  assert.equal(isDevProcess("node.exe", "\"C:\\Program Files\\nodejs\\node.exe\" server.js"), true);
  assert.equal(isDevProcess("explorer.exe", "C:\\Windows\\explorer.exe"), false);
});

test("summarizeCommand extracts meaningful script names from Windows paths", () => {
  assert.equal(
    summarizeCommand("\"C:\\Program Files\\nodejs\\node.exe\" \"C:\\work\\app\\server.js\" --port 3000", "node.exe"),
    "server.js 3000",
  );
});
