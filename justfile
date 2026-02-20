set shell := ["bash", "-c"]

default:
  @just --list

build:
  cargo build

build-release:
  cargo build --release

check:
  cargo check --all-targets

fmt:
  cargo fmt --all

fmt-check:
  cargo fmt --all -- --check

lint:
  cargo clippy --all-targets --all-features -- -D warnings

test:
  cargo test --all-targets

install-local:
  cargo install --path . --force

update-install branch="codex/m1-foundation":
  git switch "{{branch}}"
  git pull --ff-only
  cargo install --path . --force

discover:
  slipbridge printers list

discover-json:
  slipbridge printers list --json

doctor:
  slipbridge doctor

doctor-json:
  slipbridge doctor --json

smoke printer="tcp://epson-m30.local:9100":
  slipbridge printers test --printer "{{printer}}" --json

print-text printer="tcp://epson-m30.local:9100" text="Slipbridge test":
  tmp="$(mktemp)"
  trap 'rm -f "$tmp"' EXIT
  printf "%s\n" "{{text}}" > "$tmp"
  slipbridge print "$tmp" --type text --printer "{{printer}}" --paper 80mm --cut --json

print-image image_path printer="tcp://epson-m30.local:9100" paper="80mm":
  slipbridge print "{{image_path}}" --type image --printer "{{printer}}" --paper "{{paper}}" --cut --json
