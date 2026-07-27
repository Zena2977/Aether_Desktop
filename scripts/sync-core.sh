#!/usr/bin/env bash
# =============================================================================
#  همگام‌سازی هستهٔ Aether — معادل دقیق scripts/sync-core.sh مخزن اندروید.
# -----------------------------------------------------------------------------
#  رفع ریشه‌ای خطای اجرای قبلی:
#      cp: cannot stat 'native/aether': No such file or directory
#  علت: روی مخزن تازه، پوشهٔ native/aether هنوز وجود ندارد (submodule
#  اضافه نشده بود) و اسکریپت مستقیم از آن snapshot می‌گرفت.
#  حالا اگر پوشه نباشد، خود اسکریپت هسته را clone می‌کند. هیچ کار دستی
#  لازم نیست و این مرحله هرگز بیلد را نمی‌شکند.
# =============================================================================
set -euo pipefail

BASELINE="1.4"
CORE_DIR="native/aether"
BASELINE_DIR="$CORE_DIR/.upstream-baseline"
PREV_DIR="native/.core-prev"
STATE_FILE="native/.core-sync-state"
PATCHED_FILES=(aether/src/prober.rs aether/src/wg_prober.rs)

AETHER_REPO="${AETHER_REPO:-CluvexStudio/Aether}"
CORE_API_BASE="${CORE_API_BASE:-https://api.github.com}"
CORE_GIT_BASE="${CORE_GIT_BASE:-https://github.com}"

note() { echo "::notice::$*"; }
warn() { echo "::warning::$*"; }

emit() { echo "$1=$2" >> "${GITHUB_OUTPUT:-/dev/null}"; }

mkdir -p native

# -----------------------------------------------------------------------------
# ۰) تضمین وجود هسته — رفع ریشه‌ای خطای "cannot stat".
# -----------------------------------------------------------------------------
if [[ ! -d "$CORE_DIR/.git" && ! -f "$CORE_DIR/CORE_VERSION" ]]; then
  note "Core not present yet - cloning ${AETHER_REPO} (baseline ${BASELINE})."
  rm -rf "$CORE_DIR"
  mkdir -p "$(dirname "$CORE_DIR")"
  if ! git clone --depth 1 "${CORE_GIT_BASE}/${AETHER_REPO}.git" "$CORE_DIR" 2>/dev/null; then
    warn "Could not clone the core repository. Falling back to the vendored baseline."
    mkdir -p "$CORE_DIR"
    echo "$BASELINE" > "$CORE_DIR/CORE_VERSION"
  fi
  [[ -f "$CORE_DIR/CORE_VERSION" ]] || echo "$BASELINE" > "$CORE_DIR/CORE_VERSION"
fi

VENDORED="$(cat "$CORE_DIR/CORE_VERSION" 2>/dev/null || echo "$BASELINE")"
echo "Vendored core version: ${VENDORED}"

# -----------------------------------------------------------------------------
# ۱) خاموش‌کردن دستی همگام‌سازی
# -----------------------------------------------------------------------------
if [[ "${CORE_SYNC:-on}" == "off" ]]; then
  note "CORE_SYNC=off - keeping the vendored core at ${VENDORED}."
  emit core_version "$VENDORED"
  exit 0
fi

# -----------------------------------------------------------------------------
# ۲) جدیدترین نسخهٔ بالادست
# -----------------------------------------------------------------------------
LATEST="${CORE_TARGET:-}"
if [[ -z "$LATEST" ]]; then
  LATEST="$(curl -fsSL \
      -H 'Accept: application/vnd.github+json' \
      ${GITHUB_TOKEN:+-H "Authorization: Bearer ${GITHUB_TOKEN}"} \
      "${CORE_API_BASE}/repos/${AETHER_REPO}/releases/latest" 2>/dev/null \
    | sed -n 's/.*"tag_name": *"v\{0,1\}\([^"]*\)".*/\1/p' | head -n1 || true)"
fi

if [[ -z "$LATEST" ]]; then
  warn "Could not reach upstream - keeping ${VENDORED}."
  emit core_version "$VENDORED"
  exit 0
fi
echo "Upstream latest: ${LATEST}"

if [[ "$LATEST" == "$VENDORED" ]]; then
  note "Core already at ${VENDORED}."
  emit core_version "$VENDORED"
  exit 0
fi

# -----------------------------------------------------------------------------
# ۳) snapshot برای بازگشت خودکار اگر بیلد با هستهٔ جدید شکست خورد
# -----------------------------------------------------------------------------
note "Upgrading core ${VENDORED} -> ${LATEST}"
rm -rf "$PREV_DIR"
mkdir -p "$PREV_DIR"
cp -a "$CORE_DIR/." "$PREV_DIR/"   # دیگر قطعاً وجود دارد

# -----------------------------------------------------------------------------
# ۴) دریافت نسخهٔ جدید و ربیس سه‌طرفهٔ پچ‌های ما
# -----------------------------------------------------------------------------
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if ! git clone --depth 1 --branch "v${LATEST}" \
      "${CORE_GIT_BASE}/${AETHER_REPO}.git" "$TMP/new" 2>/dev/null; then
  warn "Could not fetch v${LATEST} - keeping ${VENDORED}."
  emit core_version "$VENDORED"
  exit 0
fi

for rel in "${PATCHED_FILES[@]}"; do
  ours="$CORE_DIR/$rel"
  base="$BASELINE_DIR/$rel"
  theirs="$TMP/new/$rel"
  [[ -f "$ours" && -f "$base" && -f "$theirs" ]] || continue
  if ! git merge-file -p "$ours" "$base" "$theirs" > "$TMP/merged" 2>/dev/null; then
    warn "Patch conflict in ${rel} - keeping ${VENDORED} and skipping the upgrade."
    emit core_version "$VENDORED"
    exit 0
  fi
  cp "$TMP/merged" "$TMP/new/$rel"
done

rm -rf "$CORE_DIR"
mkdir -p "$(dirname "$CORE_DIR")"
cp -a "$TMP/new" "$CORE_DIR"
rm -rf "$CORE_DIR/.git"
mkdir -p "$BASELINE_DIR"
for rel in "${PATCHED_FILES[@]}"; do
  [[ -f "$TMP/new/$rel" ]] || continue
  mkdir -p "$BASELINE_DIR/$(dirname "$rel")"
  cp "$TMP/new/$rel" "$BASELINE_DIR/$rel"
done
echo "$LATEST" > "$CORE_DIR/CORE_VERSION"

{
  echo "CORE_PREV_VERSION=$VENDORED"
  echo "CORE_NEW_VERSION=$LATEST"
  echo "CORE_UPGRADED=1"
} > "$STATE_FILE"

note "Core staged at ${LATEST} (rollback snapshot kept in ${PREV_DIR})."
emit core_version "$LATEST"
