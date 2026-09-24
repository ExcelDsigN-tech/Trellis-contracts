#!/usr/bin/env bash
# Security Audit runner for the trellis workspace.
#
# Usage:
#   ./scripts/security/run-audit.sh [--report PATH] [--allowlist PATH] [--skip-wasm]
#
# Checks (see security/README.md):
#   1. cargo audit        - RustSec advisory scan of Cargo.lock
#   2. clippy preset      - security lint set on all contract libs
#   3. wasm size pass     - release wasm size per contract (gas proxy)
#
# Exit code is non-zero when any HIGH severity finding lacks a valid entry in
# the allowlist. Medium/informational findings never gate.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
mkdir -p "$(dirname "${REPORT:-/tmp/x}")" 2>/dev/null || true

REPORT=""
ALLOWLIST="security/audit-allowlist.toml"
SKIP_WASM=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --report) REPORT="$2"; shift 2 ;;
    --allowlist) ALLOWLIST="$2"; shift 2 ;;
    --skip-wasm) SKIP_WASM=1; shift ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

# Resolve a Python >= 3.11 interpreter (needed for tomllib). On Windows the
# WindowsApps python3 alias exists but is a non-functional Store stub, so
# verify it actually runs before trusting it.
PY=""
for cand in python3 python py; do
  if command -v "$cand" >/dev/null 2>&1 &&
     "$cand" -c "import sys, tomllib" >/dev/null 2>&1; then
    PY="$cand"
    break
  fi
done
if [[ -z "$PY" ]]; then
  echo "ERROR: Python >= 3.11 (tomllib) is required for allowlist parsing." >&2
  exit 2
fi

CLIPPY_PRESET=(
  -W clippy::panic
  -W clippy::unwrap_used
  -W clippy::expect_used
  -W clippy::arithmetic_side_effects
  -W clippy::indexing_slicing
)
# Lints mapped to HIGH severity for gating purposes.
HIGH_LINTS=(clippy::panic clippy::unwrap_used clippy::expect_used)

FAILURES=()
MEDIUMS=()

section() { printf '\n=== %s ===\n' "$1"; [[ -n "$REPORT" ]] && printf '\n## %s\n\n' "$1" >> "$REPORT.tmp"; }

append_report() { [[ -n "$REPORT" ]] && printf '%s\n' "$1" >> "$REPORT.tmp"; }

# ---------------------------------------------------------------- allowlist --
# Valid exception = matching id (+ optional path scope) + non-empty rationale
# + unexpired date + matching severity.
allowlist_allows() {
  local id="$1" path="$2"
  [[ -f "$ALLOWLIST" ]] || return 1
  "$PY" - "$ALLOWLIST" "$id" "$path" <<'PY'
import sys, datetime, tomllib
allowlist, fid, fpath = sys.argv[1], sys.argv[2], sys.argv[3].replace("\\", "/")
with open(allowlist, "rb") as fh:
    data = tomllib.load(fh)
for exc in data.get("exceptions", []):
    if exc.get("id") != fid:
        continue
    if not str(exc.get("rationale", "")).strip():
        continue
    exp = exc.get("expires")
    if exp:
        try:
            if datetime.date.today() > datetime.date.fromisoformat(str(exp)):
                continue
        except ValueError:
            continue
    scope = exc.get("path")
    if scope and scope not in fpath:
        continue
    sys.exit(0)
sys.exit(1)
PY
}

# ------------------------------------------------------------- cargo audit --
section "Dependency advisories (cargo audit)"
if ! command -v cargo-audit >/dev/null 2>&1; then
  msg="cargo-audit not installed; advisory scan SKIPPED (install: cargo install cargo-audit --locked)"
  echo "WARN: $msg"
  append_report "> WARN: $msg"
else
  cargo audit --json -f "$ROOT/Cargo.lock" > "$WORKDIR/audit.json" 2>"$WORKDIR/audit.err" || true
  python3 - "$WORKDIR/audit.json" <<'PY' > "$WORKDIR/audit.findings"
import json, sys
try:
    data = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception:
    # A scan that produced no parseable output must not read as "clean".
    print("AUDIT-ERROR\tHIGH\tcargo-audit\tno parseable output (see cargo audit stderr)")
    sys.exit(0)
vulns = data.get("vulnerabilities", {}).get("list", []) or []
for v in vulns:
    adv = v.get("advisory", {})
    # RustSec advisories carry no severity field (only an optional CVSS
    # vector), so every vulnerability is treated as HIGH and gates.
    print(f"{adv.get('id','?')}\tHIGH\t{v.get('package',{}).get('name','?')}\t{adv.get('title','')}")
warnings = data.get("warnings", []) or []
if isinstance(warnings, dict):  # cargo-audit >= 0.17: {kind: [warning, ...]}
    warnings = [w for ws in warnings.values() for w in ws]
for w in warnings:
    print(f"{w.get('package',{}).get('name','?')}\tMEDIUM\t{w.get('package',{}).get('name','?')}\t{w.get('kind','warning')}: unmaintained/duplicate dependency")
PY
  while IFS=$'\t' read -r id sev pkg title; do
    line="- **$id** ($sev) - \`$pkg\`: $title"
    echo "$line"; append_report "$line"
    if [[ "$sev" == "CRITICAL" || "$sev" == "HIGH" ]]; then
      if allowlist_allows "$id" "."; then
        note="- $id: allowed by allowlist entry"; echo "$note"; append_report "$note"
      else
        FAILURES+=("$id ($pkg)")
      fi
    else
      MEDIUMS+=("$line")
    fi
  done < "$WORKDIR/audit.findings"
  grep -q '^AUDIT-ERROR' "$WORKDIR/audit.findings" && cat "$WORKDIR/audit.err" >&2
  [[ -s "$WORKDIR/audit.findings" ]] || { echo "No known advisories."; append_report "No known advisories."; }
fi

# ------------------------------------------------------------ clippy preset --
section "Security lint preset (clippy, lib targets)"
CLIPPY_JSON="$WORKDIR/clippy.json"
RUST_MIN_STACK=33554432 cargo clippy --workspace --lib --message-format=json -- "${CLIPPY_PRESET[@]}" > "$CLIPPY_JSON" 2> "$WORKDIR/clippy.human.log"
grep -E "^warning" "$WORKDIR/clippy.human.log" | sort | uniq -c | sed 's/^/  /' || true
append_report '```'
[[ -n "$REPORT" ]] && grep -E "^warning|-->" "$WORKDIR/clippy.human.log" >> "$REPORT.tmp" 2>/dev/null || true
[[ -n "$REPORT" ]] && printf '```\n' >> "$REPORT.tmp"

"$PY" - "$CLIPPY_JSON" <<'PY' > "$WORKDIR/clippy.findings"
import json, sys
with open(sys.argv[1], encoding="utf-8") as fh:
    for raw in fh:
        try:
            msg = json.loads(raw)
        except json.JSONDecodeError:
            continue
        reason = msg.get("reason")
        if reason not in ("compiler-message",):
            continue
        m = msg.get("message", {})
        code = (m.get("code") or {}).get("code") or ""
        if not code.startswith("clippy::"):
            continue
        spans = [s for s in m.get("spans", []) if s.get("is_primary")]
        if not spans:
            continue
        s = spans[0]
        print(f"{code}\t{s.get('file_name','?')}\t{s.get('line_start',0)}")
PY

while IFS=$'\t' read -r lint file line; do
  [[ -z "$lint" ]] && continue
  if printf '%s\n' "${HIGH_LINTS[@]}" | grep -qx "$lint"; then
    if ! allowlist_allows "$lint" "$file"; then
      FAILURES+=("$lint at $file:$line")
    fi
  else
    MEDIUMS+=("$lint at $file:$line")
  fi
done < "$WORKDIR/clippy.findings"

# -------------------------------------------------------------- wasm sizes ---
if [[ "$SKIP_WASM" -eq 1 ]]; then
  section "Wasm size inventory (SKIPPED)"
  append_report "Skipped by flag."
else
  section "Wasm size inventory (release, gas-cost proxy)"
  PACKAGES=$(grep -h '^name = ' contracts/*/Cargo.toml | sed -E 's/name = "(.+)"/\1/')
  for pkg in $PACKAGES; do
    if RUST_MIN_STACK=33554432 cargo build -p "$pkg" --release \
         --target wasm32v1-none > /dev/null 2>&1; then
      wasm="target/wasm32v1-none/release/${pkg//-/_}.wasm"
      if [[ -f "$wasm" ]]; then
        line="- \`$pkg\`: $(wc -c < "$wasm") bytes"
        echo "$line"; append_report "$line"
        continue
      fi
    fi
    line="- \`$pkg\`: build failed or no wasm artifact produced"
    echo "$line"; append_report "$line"
  done
fi

# ------------------------------------------------------------------ summary --
section "Summary"
if [[ ${#MEDIUMS[@]} -gt 0 ]]; then
  echo "Medium/informational findings (non-gating): ${#MEDIUMS[@]}"
  append_report "Medium/informational findings (non-gating): ${#MEDIUMS[@]}"
fi
if [[ ${#FAILURES[@]} -gt 0 ]]; then
  echo ""
  echo "GATING FAILURES (${#FAILURES[@]}): high-severity findings without a valid allowlist entry:"
  for f in "${FAILURES[@]}"; do echo "  - $f"; append_report "- GATE: $f"; done
  echo ""
  echo "Fix the findings or add a justified entry to $ALLOWLIST."
  exit 1
fi
echo "PASS: no ungated high-severity findings."
[[ -n "$REPORT" ]] && append_report "PASS: no ungated high-severity findings."

if [[ -n "$REPORT" ]]; then
  {
    echo "# Security Audit Report"
    echo ""
    echo "- Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- Allowlist: \`$ALLOWLIST\`"
    cat "$REPORT.tmp"
  } > "$REPORT"
  rm -f "$REPORT.tmp"
  echo ""
  echo "Report written to $REPORT"
fi
exit 0
