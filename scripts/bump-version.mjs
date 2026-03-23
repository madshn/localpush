#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const packageJsonPath = path.join(root, "package.json");
const cargoTomlPath = path.join(root, "src-tauri", "Cargo.toml");
const cargoLockPath = path.join(root, "src-tauri", "Cargo.lock");
const tauriConfigPath = path.join(root, "src-tauri", "tauri.conf.json");

const usage = `Usage:
  npm run bump:version -- <major|minor|patch|x.y.z> [--dry-run]

Examples:
  npm run bump:version -- patch
  npm run bump:version -- minor
  npm run bump:version -- 0.6.0
`;

function fail(message) {
  console.error(message);
  process.exit(1);
}

function isSemver(value) {
  return /^\d+\.\d+\.\d+$/.test(value);
}

function bumpSemver(version, kind) {
  const match = version.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) {
    fail(`Current version is not simple semver: ${version}`);
  }

  let [major, minor, patch] = match.slice(1).map(Number);

  if (kind === "major") {
    major += 1;
    minor = 0;
    patch = 0;
  } else if (kind === "minor") {
    minor += 1;
    patch = 0;
  } else if (kind === "patch") {
    patch += 1;
  } else {
    fail(`Unknown bump kind: ${kind}`);
  }

  return `${major}.${minor}.${patch}`;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function extractCargoVersion(cargoToml) {
  const match = cargoToml.match(/^version = "([^"]+)"$/m);
  if (!match) {
    fail("Could not find package version in src-tauri/Cargo.toml");
  }
  return match[1];
}

function replaceCargoVersion(cargoToml, nextVersion) {
  return cargoToml.replace(/^version = "[^"]+"$/m, `version = "${nextVersion}"`);
}

function replaceCargoLockVersion(cargoLock, nextVersion) {
  const pattern = /(\[\[package\]\]\nname = "localpush"\nversion = ")([^"]+)(")/;
  if (!pattern.test(cargoLock)) {
    fail('Could not find localpush package block in src-tauri/Cargo.lock');
  }
  return cargoLock.replace(pattern, `$1${nextVersion}$3`);
}

function parseBuildNumber(value) {
  if (value == null) {
    return 0;
  }
  if (!/^\d+$/.test(String(value))) {
    fail(
      `Expected bundle.macOS.bundleVersion to be an integer string, got: ${value}`,
    );
  }
  return Number(value);
}

const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const positional = args.filter((arg) => arg !== "--dry-run");
const requested = positional[0];

if (!requested) {
  fail(usage);
}

const packageJson = readJson(packageJsonPath);
const tauriConfig = readJson(tauriConfigPath);
const cargoToml = fs.readFileSync(cargoTomlPath, "utf8");
const cargoLock = fs.readFileSync(cargoLockPath, "utf8");

const currentVersion = packageJson.version;
const cargoVersion = extractCargoVersion(cargoToml);
const tauriVersion = tauriConfig.version;

if (!isSemver(currentVersion)) {
  fail(`package.json version is not simple semver: ${currentVersion}`);
}

if (cargoVersion !== currentVersion || tauriVersion !== currentVersion) {
  fail(
    `Version mismatch detected:\n` +
      `- package.json: ${currentVersion}\n` +
      `- src-tauri/Cargo.toml: ${cargoVersion}\n` +
      `- src-tauri/tauri.conf.json: ${tauriVersion}`,
  );
}

const nextVersion = isSemver(requested)
  ? requested
  : bumpSemver(currentVersion, requested);

const currentBuildNumber = parseBuildNumber(tauriConfig.bundle?.macOS?.bundleVersion);
const nextBuildNumber = String(currentBuildNumber + 1);

if (!tauriConfig.bundle) {
  tauriConfig.bundle = {};
}
if (!tauriConfig.bundle.macOS) {
  tauriConfig.bundle.macOS = {};
}

packageJson.version = nextVersion;
tauriConfig.version = nextVersion;
tauriConfig.bundle.macOS.bundleVersion = nextBuildNumber;

const nextCargoToml = replaceCargoVersion(cargoToml, nextVersion);
const nextCargoLock = replaceCargoLockVersion(cargoLock, nextVersion);

if (!dryRun) {
  writeJson(packageJsonPath, packageJson);
  writeJson(tauriConfigPath, tauriConfig);
  fs.writeFileSync(cargoTomlPath, nextCargoToml);
  fs.writeFileSync(cargoLockPath, nextCargoLock);
}

console.log(`App version:       ${currentVersion} -> ${nextVersion}`);
console.log(`macOS build no.:   ${currentBuildNumber} -> ${nextBuildNumber}`);
if (dryRun) {
  console.log("Dry run only; no files were changed.");
}
