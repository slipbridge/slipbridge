#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');

function fail(message) {
  console.error(message);
  process.exit(1);
}

const nextVersion = process.argv[2];
if (!nextVersion) {
  fail('usage: node scripts/bump-version.js <version>');
}

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(nextVersion)) {
  fail(`invalid version '${nextVersion}' (expected semver like 0.1.0)`);
}

const repoRoot = path.resolve(__dirname, '..');
const cargoTomlPath = path.join(repoRoot, 'Cargo.toml');
const npmRootPath = path.join(repoRoot, 'npm');

function writeIfChanged(filePath, nextContent) {
  const current = fs.readFileSync(filePath, 'utf8');
  if (current === nextContent) {
    return false;
  }

  fs.writeFileSync(filePath, nextContent);
  return true;
}

function updateCargoToml(filePath, version) {
  const source = fs.readFileSync(filePath, 'utf8');
  const lines = source.split('\n');

  let inPackageSection = false;
  let replaced = false;

  for (let i = 0; i < lines.length; i += 1) {
    const trimmed = lines[i].trim();

    if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
      inPackageSection = trimmed === '[package]';
      continue;
    }

    if (inPackageSection && /^version\s*=\s*"[^"]+"\s*$/.test(trimmed)) {
      lines[i] = `version = "${version}"`;
      replaced = true;
      break;
    }
  }

  if (!replaced) {
    fail('failed to locate [package] version in Cargo.toml');
  }

  const output = `${lines.join('\n').replace(/\n?$/, '\n')}`;
  return writeIfChanged(filePath, output);
}

function collectPackageJsonFiles(npmRoot) {
  const result = [];

  function walk(current) {
    const entries = fs.readdirSync(current, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath);
      } else if (entry.isFile() && entry.name === 'package.json') {
        result.push(fullPath);
      }
    }
  }

  walk(npmRoot);
  result.sort();
  return result;
}

function updatePackageJsonText(source, version, isRootPackage) {
  let output = source.replace(
    /("version"\s*:\s*")[^"]+(")/,
    `$1${version}$2`,
  );

  if (isRootPackage) {
    output = output.replace(
      /("@slipbridge\/cli-[^"]+"\s*:\s*")[^"]+(")/g,
      `$1${version}$2`,
    );
  }

  return output;
}

const changedFiles = [];

if (updateCargoToml(cargoTomlPath, nextVersion)) {
  changedFiles.push(cargoTomlPath);
}

if (!fs.existsSync(npmRootPath)) {
  fail('npm directory not found');
}

const packageJsonFiles = collectPackageJsonFiles(npmRootPath);
if (packageJsonFiles.length === 0) {
  fail('no package.json files found under npm/');
}

for (const packagePath of packageJsonFiles) {
  const source = fs.readFileSync(packagePath, 'utf8');
  const isRootPackage = packagePath.endsWith(
    `${path.sep}npm${path.sep}slipbridge${path.sep}package.json`,
  );
  const output = updatePackageJsonText(source, nextVersion, isRootPackage);
  if (writeIfChanged(packagePath, output)) {
    changedFiles.push(packagePath);
  }
}

if (changedFiles.length === 0) {
  console.log(`versions already set to ${nextVersion}`);
} else {
  console.log(`bumped versions to ${nextVersion} (${changedFiles.length} file(s) changed)`);
}
