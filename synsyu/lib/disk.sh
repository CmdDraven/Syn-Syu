#============================================================
# Synavera Project: Syn-Syu
# Module: synsyu/lib/disk.sh
# Etiquette: Synavera Script Etiquette — Bash Profile v1.1.1
#------------------------------------------------------------
# Purpose:
#   Disk space assessment utilities for manifest-driven updates.
#
# Security / Safety Notes:
#   Reads manifest JSON and disk usage via df; no privileged
#   writes beyond invoking df and parsing.
#------------------------------------------------------------
# SSE Principles Observed:
#   - Guardrails around disk operations with explicit logging
#   - Pure calculations separated from orchestration logic
#------------------------------------------------------------

#--- format_bytes
format_bytes() {
  local bytes="${1:-0}"
  if ! [[ "$bytes" =~ ^[0-9]+$ ]]; then
    bytes=0
  fi
  if command -v numfmt >/dev/null 2>&1; then
    numfmt --to=iec --suffix=B "$bytes"
  else
    printf '%sB' "$bytes"
  fi
}

#--- check_disk_space
check_disk_space() {
  [ "$DISK_CHECK" = "1" ] || return 0
  local manifest_metrics
  manifest_metrics="$(python3 - "$SYN_MANIFEST_PATH" <<'PY' 2>/dev/null
import json
import sys

try:
    with open(sys.argv[1], "r", encoding="utf-8") as handle:
        data = json.load(handle)
except FileNotFoundError:
    sys.exit(1)
metadata = data.get("metadata") or {}

def to_int(value):
    if isinstance(value, (int, float)):
        if value < 0:
            return 0
        return int(value)
    return 0

download = to_int(metadata.get("download_size_total"))
build = to_int(metadata.get("build_size_total"))
install = to_int(metadata.get("install_size_total"))
transient = to_int(metadata.get("transient_size_total"))
margin = to_int(metadata.get("min_free_bytes"))
available = to_int(metadata.get("available_space_bytes"))
path = metadata.get("space_checked_path") or ""

if transient == 0:
    transient = download + build + install

print(f"{download}|{build}|{install}|{transient}|{margin}|{available}|{path}")
PY
)"
  if [ -z "$manifest_metrics" ]; then
    log_warn "DISK" "Manifest lacks space metadata; skipping disk check"
    return 0
  fi

  local download_bytes build_bytes install_bytes transient_bytes manifest_margin available_bytes recorded_path
  IFS='|' read -r download_bytes build_bytes install_bytes transient_bytes manifest_margin available_bytes recorded_path <<<"$manifest_metrics"

  download_bytes="${download_bytes:-0}"
  build_bytes="${build_bytes:-0}"
  install_bytes="${install_bytes:-0}"
  transient_bytes="${transient_bytes:-0}"
  manifest_margin="${manifest_margin:-0}"
  available_bytes="${available_bytes:-0}"
  recorded_path="${recorded_path:-/}"

  if ! [[ "$download_bytes" =~ ^[0-9]+$ && "$build_bytes" =~ ^[0-9]+$ && "$install_bytes" =~ ^[0-9]+$ ]]; then
    log_warn "DISK" "Manifest space metrics invalid; skipping disk check"
    return 0
  fi
  if ! [[ "$transient_bytes" =~ ^[0-9]+$ ]]; then
    transient_bytes=$((download_bytes + build_bytes + install_bytes))
  fi
  if [ "$transient_bytes" -eq 0 ]; then
    log_info "DISK" "No updates require disk resources."
    return 0
  fi

  local margin_bytes="$MIN_FREE_SPACE_BYTES"
  local extra_margin_bytes=$((DISK_MARGIN_MB * 1024 * 1024))
  margin_bytes=$((margin_bytes + extra_margin_bytes))
  if [[ "$manifest_margin" =~ ^[0-9]+$ ]] && [ "$manifest_margin" -gt "$margin_bytes" ]; then
    margin_bytes="$manifest_margin"
  fi
  local required_bytes=$((transient_bytes + margin_bytes))

  local available_path="$recorded_path"
  if [ -z "$available_path" ]; then
    available_path="/"
  fi
  if ! [[ "$available_bytes" =~ ^[0-9]+$ ]] || [ "$available_bytes" -eq 0 ]; then
    available_bytes="$(df -Pk "$available_path" | awk 'NR==2 {print $4 * 1024}')"
  fi
  if ! [[ "$available_bytes" =~ ^[0-9]+$ ]]; then
    log_warn "DISK" "Unable to read available disk space; skipping check"
    return 0
  fi
  SPACE_CHECK_PATH="$available_path"

  if [ "$available_bytes" -lt "$required_bytes" ]; then
    log_error "DISK" "Insufficient space: need $(format_bytes "$required_bytes") (download $(format_bytes "$download_bytes") + build $(format_bytes "$build_bytes") + install $(format_bytes "$install_bytes") + buffer $(format_bytes "$margin_bytes")), only $(format_bytes "$available_bytes") available on $available_path"
    log_finalize
    exit 421
  fi

  log_info "DISK" "Disk space check passed: need $(format_bytes "$required_bytes") (download $(format_bytes "$download_bytes") + build $(format_bytes "$build_bytes") + install $(format_bytes "$install_bytes") + buffer $(format_bytes "$margin_bytes")), have $(format_bytes "$available_bytes") on $available_path"
}

#--- ensure_package_disk_space
ensure_package_disk_space() {
  [ "$DISK_CHECK" = "1" ] || return 0
  local pkg="$1"
  local metrics
  metrics="$(manifest_package_requirements "$pkg" || true)"
  if [ -z "$metrics" ]; then
    log_warn "DISK" "No manifest metrics available for $pkg; skipping disk verification"
    return 0
  fi
  local download_bytes build_bytes install_bytes transient_bytes
  IFS='|' read -r download_bytes build_bytes install_bytes transient_bytes <<<"$metrics"
  download_bytes="${download_bytes:-0}"
  build_bytes="${build_bytes:-0}"
  install_bytes="${install_bytes:-0}"
  transient_bytes="${transient_bytes:-0}"
  if ! [[ "$download_bytes" =~ ^[0-9]+$ ]]; then
    download_bytes=0
  fi
  if ! [[ "$build_bytes" =~ ^[0-9]+$ ]]; then
    build_bytes=0
  fi
  if ! [[ "$install_bytes" =~ ^[0-9]+$ ]]; then
    install_bytes=0
  fi
  if ! [[ "$transient_bytes" =~ ^[0-9]+$ ]]; then
    transient_bytes=0
  fi
  if [ "$download_bytes" -eq 0 ] && [ "$build_bytes" -eq 0 ] && [ "$install_bytes" -eq 0 ]; then
    log_warn "DISK" "Package $pkg lacks size telemetry; continuing without disk guard"
    return 0
  fi
  local margin_bytes="$MIN_FREE_SPACE_BYTES"
  local extra_margin_bytes=$((DISK_MARGIN_MB * 1024 * 1024))
  margin_bytes=$((margin_bytes + extra_margin_bytes))
  local required_bytes
  if [ "$transient_bytes" -gt 0 ]; then
    required_bytes="$transient_bytes"
  else
    required_bytes=$((download_bytes + build_bytes + install_bytes))
  fi
  local total_needed=$((required_bytes + margin_bytes))
  local available_bytes
  available_bytes="$(df -Pk "$SPACE_CHECK_PATH" | awk 'NR==2 {print $4 * 1024}')"
  if ! [[ "$available_bytes" =~ ^[0-9]+$ ]]; then
    log_warn "DISK" "Unable to assess disk space prior to installing $pkg"
    return 0
  fi
  if [ "$available_bytes" -lt "$total_needed" ]; then
    log_error "DISK" "Skipping $pkg: requires $(format_bytes "$total_needed") (download $(format_bytes "$download_bytes"), build $(format_bytes "$build_bytes"), install $(format_bytes "$install_bytes"), buffer $(format_bytes "$margin_bytes")) but only $(format_bytes "$available_bytes") available on $SPACE_CHECK_PATH"
    return 1
  fi
  log_debug "DISK" "Sufficient space for $pkg (need $(format_bytes "$total_needed"), available $(format_bytes "$available_bytes") on $SPACE_CHECK_PATH)"
  return 0
}
