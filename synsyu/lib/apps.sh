#============================================================
# Synavera Project: Syn-Syu
# Module: synsyu/lib/apps.sh
# Etiquette: Synavera Script Etiquette — Bash Profile v1.1
#------------------------------------------------------------
# Purpose:
#   Application update helpers for Flatpak and fwupd.
#
# Security / Safety Notes:
#   Executes flatpak/fwupdmgr as invoking user; may request
#   privilege elevation internally per tool behavior.
#------------------------------------------------------------
# SSE Principles Observed:
#   - Clear separation of app-specific update flows
#   - Defensive logging and failure tracking
#------------------------------------------------------------

#--- run_flatpak_updates
run_flatpak_updates() {
  if ! command -v flatpak >/dev/null 2>&1; then
    log_warn "FLATPAK" "flatpak not installed; skipping Flatpak updates"
    return 0
  fi

  if [ "$DRY_RUN" = "1" ]; then
    local updates
    updates="$(flatpak_dry_run_list)"
    if [ -z "$updates" ]; then
      log_info "FLATPAK" "No Flatpak updates available (dry-run)"
    else
      log_info "FLATPAK" "Pending Flatpak updates (dry-run):"
      printf '%s\n' "$updates"
    fi
    return 0
  fi

  log_info "FLATPAK" "Applying Flatpak updates"
  # Security: flatpak may prompt for privileges per system policy; invoked non-interactively when supported.
  local -a fp_args=(update)
  # Prefer non-interactive, yes-to-all when available.
  if flatpak --help 2>/dev/null | grep -q -- "--noninteractive"; then
    fp_args+=(--noninteractive)
  fi
  fp_args+=(--assumeyes)
  if ! flatpak "${fp_args[@]}"; then
    local status=$?
    record_failed_update "flatpak" "flatpak update failed (exit $status)"
    # Fallback without noninteractive for older Flatpak versions.
    if ! flatpak update --assumeyes; then
      status=$?
      record_failed_update "flatpak" "flatpak update failed (exit $status)"
      log_error "FLATPAK" "Flatpak update failed"
      return 1
    fi
  fi
  return 0
}

flatpak_dry_run_list() {
  local updates_output
  updates_output="$(flatpak remote-ls --updates --columns=application,branch,origin 2>/dev/null || true)"
  printf '%s' "$updates_output"
}

#--- run_fwupd_updates
run_fwupd_updates() {
  if ! command -v fwupdmgr >/dev/null 2>&1; then
    log_warn "FWUPD" "fwupdmgr not installed; skipping firmware updates"
    return 0
  fi

  if [ "$DRY_RUN" = "1" ]; then
    local output
    output="$(fwupdmgr get-updates 2>/dev/null || true)"
    if [ -z "$output" ]; then
      log_info "FWUPD" "No firmware updates available (dry-run)"
    else
      log_info "FWUPD" "Pending firmware updates (dry-run):"
      printf '%s\n' "$output"
    fi
    return 0
  fi

  log_info "FWUPD" "Applying firmware updates via fwupdmgr"
  # Security: fwupdmgr may use system firmware channels; run as user and respect tool prompts.
  local -a args=(update)
  if fwupdmgr --help 2>/dev/null | grep -q -- "--assume-yes"; then
    args+=(--assume-yes)
  fi
  if ! fwupdmgr "${args[@]}"; then
    local status=$?
    record_failed_update "fwupd" "fwupdmgr update failed (exit $status)"
    log_error "FWUPD" "Firmware update failed"
    return 1
  fi
  return 0
}
