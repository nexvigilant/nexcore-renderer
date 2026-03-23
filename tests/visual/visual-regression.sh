#!/usr/bin/env bash
set -euo pipefail

# ============================================================================
# NexBrowser Visual Regression Harness
#
# Renders HTML fixtures → captures screenshots via debug server → compares
# against baselines using ImageMagick pixel diff.
#
# Usage:
#   ./visual-regression.sh                # Run regression (compare vs baselines)
#   ./visual-regression.sh --update       # Generate/update baselines
#   ./visual-regression.sh --fixture 02   # Run single fixture by prefix
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="${SCRIPT_DIR}/fixtures"
BASELINES_DIR="${SCRIPT_DIR}/baselines"
DIFFS_DIR="${SCRIPT_DIR}/diffs"
SCREENSHOTS_DIR="${SCRIPT_DIR}/screenshots"

# Derive binary path relative to this script's location.
# Script lives at: <workspace>/crates/nexcore-renderer/tests/visual/
# Binary lives at: <workspace>/target/debug/nexbrowser
# Allow override via NEXBROWSER env var for CI.
NEXBROWSER="${NEXBROWSER:-$(cd "${SCRIPT_DIR}/../../../.." && pwd)/target/debug/nexbrowser}"
DEBUG_PORT=9333
SCREENSHOT_URL="http://localhost:${DEBUG_PORT}/screenshot.png"
HEALTH_URL="http://localhost:${DEBUG_PORT}/health"

# Pixel diff threshold (0 = identical, higher = more tolerance)
# RMSE threshold — values below this pass
THRESHOLD=500

UPDATE_MODE=false
FILTER=""

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --update)  UPDATE_MODE=true; shift ;;
        --fixture) FILTER="$2"; shift 2 ;;
        --threshold) THRESHOLD="$2"; shift 2 ;;
        *) echo "[!] unknown arg: $1"; exit 1 ;;
    esac
done

# Colors
C_CYAN='\033[0;36m'
C_GREEN='\033[0;32m'
C_RED='\033[0;31m'
C_YELLOW='\033[0;33m'
C_GRAY='\033[0;90m'
C_RESET='\033[0m'

pass()  { echo -e "  ${C_GREEN}[PASS]${C_RESET} $1"; }
fail()  { echo -e "  ${C_RED}[FAIL]${C_RESET} $1"; FAILURES=$((FAILURES + 1)); }
warn()  { echo -e "  ${C_YELLOW}[WARN]${C_RESET} $1"; }
info()  { echo -e "  ${C_GRAY}[....]${C_RESET} $1"; }

FAILURES=0
TOTAL=0

# Ensure dirs exist
mkdir -p "${BASELINES_DIR}" "${DIFFS_DIR}" "${SCREENSHOTS_DIR}"

# Check prerequisites
if [[ ! -f "${NEXBROWSER}" ]]; then
    echo -e "${C_RED}[!] nexbrowser binary not found at ${NEXBROWSER}${C_RESET}"
    echo "    Run: cargo build -p nexcore-renderer"
    exit 1
fi

if ! command -v compare &>/dev/null; then
    echo -e "${C_RED}[!] ImageMagick 'compare' not found${C_RESET}"
    exit 1
fi

if ! command -v convert &>/dev/null; then
    echo -e "${C_RED}[!] ImageMagick 'convert' not found${C_RESET}"
    exit 1
fi

# Kill any existing nexbrowser on our debug port
cleanup() {
    if [[ -n "${BROWSER_PID:-}" ]]; then
        kill "${BROWSER_PID}" 2>/dev/null || true
        wait "${BROWSER_PID}" 2>/dev/null || true
    fi
}
trap cleanup EXIT

wait_for_debug_server() {
    local max_wait=15
    local waited=0
    while ! curl -sf "${HEALTH_URL}" &>/dev/null; do
        sleep 0.5
        waited=$((waited + 1))
        if [[ ${waited} -ge $((max_wait * 2)) ]]; then
            return 1
        fi
    done
    return 0
}

wait_for_render() {
    # Give browser time to parse + layout + render the page
    sleep 2
}

capture_screenshot() {
    local output_path="$1"
    curl -sf -o "${output_path}" "${SCREENSHOT_URL}"
}

launch_browser() {
    local url="$1"
    # Kill previous instance if any
    if [[ -n "${BROWSER_PID:-}" ]]; then
        kill "${BROWSER_PID}" 2>/dev/null || true
        wait "${BROWSER_PID}" 2>/dev/null || true
        sleep 0.5
    fi

    DISPLAY=:0 "${NEXBROWSER}" "${url}" &>/dev/null &
    BROWSER_PID=$!

    if ! wait_for_debug_server; then
        fail "debug server did not start for ${url}"
        return 1
    fi
    return 0
}

# Header
echo ""
echo -e "${C_CYAN}============================================${C_RESET}"
if ${UPDATE_MODE}; then
    echo -e "${C_CYAN}  NexBrowser Visual Regression — UPDATE${C_RESET}"
else
    echo -e "${C_CYAN}  NexBrowser Visual Regression — TEST${C_RESET}"
fi
echo -e "${C_CYAN}============================================${C_RESET}"
echo ""

# Collect fixtures
FIXTURE_FILES=()
for f in "${FIXTURES_DIR}"/*.html; do
    [[ -f "$f" ]] || continue
    if [[ -n "${FILTER}" ]]; then
        basename_f="$(basename "$f")"
        if [[ "${basename_f}" != "${FILTER}"* ]]; then
            continue
        fi
    fi
    FIXTURE_FILES+=("$f")
done

if [[ ${#FIXTURE_FILES[@]} -eq 0 ]]; then
    echo -e "${C_RED}[!] no fixtures found${C_RESET}"
    exit 1
fi

echo -e "  ${C_GRAY}fixtures: ${#FIXTURE_FILES[@]}  threshold: ${THRESHOLD}  mode: $(${UPDATE_MODE} && echo UPDATE || echo TEST)${C_RESET}"
echo ""

for fixture in "${FIXTURE_FILES[@]}"; do
    name="$(basename "${fixture}" .html)"
    TOTAL=$((TOTAL + 1))

    echo -e "${C_CYAN}--- ${name} ---${C_RESET}"

    # Convert to file:// URL
    file_url="file://${fixture}"

    # Launch browser with this fixture
    if ! launch_browser "${file_url}"; then
        continue
    fi

    wait_for_render

    # Capture screenshot
    screenshot="${SCREENSHOTS_DIR}/${name}.png"
    if ! capture_screenshot "${screenshot}"; then
        fail "screenshot capture failed"
        continue
    fi

    # Verify screenshot is a valid PNG (>100 bytes)
    file_size=$(stat -c%s "${screenshot}" 2>/dev/null || echo 0)
    if [[ ${file_size} -lt 100 ]]; then
        fail "screenshot too small (${file_size} bytes)"
        continue
    fi

    info "captured ${file_size} bytes"

    if ${UPDATE_MODE}; then
        # Update mode: save as baseline
        cp "${screenshot}" "${BASELINES_DIR}/${name}.png"
        pass "baseline updated (${file_size} bytes)"
    else
        # Test mode: compare against baseline
        baseline="${BASELINES_DIR}/${name}.png"
        if [[ ! -f "${baseline}" ]]; then
            warn "no baseline — run with --update first"
            continue
        fi

        diff_img="${DIFFS_DIR}/${name}-diff.png"

        # ImageMagick compare: outputs RMSE to stderr
        # AE = Absolute Error (pixel count), RMSE = Root Mean Square Error
        set +e
        metric_output=$(compare -metric RMSE "${baseline}" "${screenshot}" "${diff_img}" 2>&1)
        compare_exit=$?
        set -e

        # Extract numeric RMSE (first number before parenthesis)
        rmse=$(echo "${metric_output}" | grep -oP '[\d.]+' | head -1)
        rmse_int=${rmse%%.*}  # truncate to integer

        if [[ -z "${rmse_int}" ]]; then
            rmse_int=99999
        fi

        if [[ ${rmse_int} -le ${THRESHOLD} ]]; then
            pass "RMSE=${rmse} (threshold=${THRESHOLD})"
            rm -f "${diff_img}"  # clean up passing diffs
        else
            fail "RMSE=${rmse} > threshold=${THRESHOLD} — diff: ${diff_img}"
        fi
    fi

    echo ""
done

# Kill browser
cleanup

# Summary
echo -e "${C_CYAN}============================================${C_RESET}"
if ${UPDATE_MODE}; then
    echo -e "  ${C_GREEN}${TOTAL} baselines updated${C_RESET}"
else
    PASSED=$((TOTAL - FAILURES))
    if [[ ${FAILURES} -eq 0 ]]; then
        echo -e "  ${C_GREEN}${PASSED}/${TOTAL} passed — all clear${C_RESET}"
    else
        echo -e "  ${C_RED}${PASSED}/${TOTAL} passed — ${FAILURES} failed${C_RESET}"
        echo -e "  ${C_GRAY}check diffs in: ${DIFFS_DIR}/${C_RESET}"
    fi
fi
echo -e "${C_CYAN}============================================${C_RESET}"

exit ${FAILURES}
