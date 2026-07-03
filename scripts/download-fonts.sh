#!/usr/bin/env bash

# =============================================================================
# Font Download Script for Zaroxi Studio (pure-Rust)
#
# Downloads the JetBrains Mono Nerd Font and installs the variants into
# `assets/fonts/`, the workspace-bundled font directory read at runtime by
# `zaroxi-core-engine-font` (see `load_project_font_bytes`). The engine PREFERS
# the "Mono" Nerd Font variant (single-cell icon width) and falls back to the
# standard variant, so both are installed.
#
# There is no Tauri / web frontend — fonts are consumed directly by the Rust
# rendering stack (cosmic-text / wgpu / vello).
# =============================================================================

set -euo pipefail

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------
readonly SCRIPT_NAME=$(basename "$0")
readonly SCRIPT_VERSION="2.0.0"

# Resolve the workspace root from this script's location so the script works
# regardless of the current working directory.
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly FONT_DIR="${REPO_ROOT}/assets/fonts"

readonly NERD_FONTS_REPO="https://github.com/ryanoasis/nerd-fonts/releases/download/v3.4.0"
readonly ZIP_FILE="JetBrainsMono.zip"
readonly DOWNLOAD_URL="${NERD_FONTS_REPO}/${ZIP_FILE}"

# Files to install (source basename inside the archive == destination basename).
# The engine's font loader looks for these exact names under assets/fonts/:
#   - JetBrainsMonoNerdFontMono-Regular.ttf   (preferred: single-cell icons)
#   - JetBrainsMonoNerdFont-Regular.ttf       (fallback)
# The additional weights/styles are installed for future use by the renderer.
readonly FONT_FILES=(
    "JetBrainsMonoNerdFontMono-Regular.ttf"
    "JetBrainsMonoNerdFont-Regular.ttf"
    "JetBrainsMonoNerdFont-Bold.ttf"
    "JetBrainsMonoNerdFont-Italic.ttf"
    "JetBrainsMonoNerdFont-BoldItalic.ttf"
)

# -----------------------------------------------------------------------------
# Logging functions
# -----------------------------------------------------------------------------
log_info()    { echo "[INFO] $*"; }
log_success() { echo "✅ $*"; }
log_warning() { echo "⚠️  $*"; }
log_error()   { echo "❌ $*" >&2; }
log_debug()   { if [[ "${DEBUG:-false}" == "true" ]]; then echo "[DEBUG] $*"; fi; }

# -----------------------------------------------------------------------------
# Utility functions
# -----------------------------------------------------------------------------
cleanup() {
    local exit_code=$?
    if [[ -n "${TEMP_DIR:-}" && -d "$TEMP_DIR" ]]; then
        log_debug "Removing temporary directory: $TEMP_DIR"
        rm -rf "$TEMP_DIR"
    fi
    if [[ $exit_code -eq 0 ]]; then
        log_success "Script completed successfully"
    else
        log_error "Script failed with exit code $exit_code"
    fi
}

print_usage() {
    cat << EOF
Usage: $SCRIPT_NAME [OPTIONS]

Downloads and installs the JetBrains Mono Nerd Font into assets/fonts/ for the
pure-Rust Zaroxi Studio rendering stack.

Options:
    -h, --help      Show this help message
    -v, --version   Show version information
    -d, --debug     Enable debug output
    --clean         Remove existing .ttf files in assets/fonts/ before installing

Examples:
    $SCRIPT_NAME              # Download and install fonts
    $SCRIPT_NAME --clean      # Clean install
    $SCRIPT_NAME --debug      # Enable debug output
EOF
}

print_version() { echo "$SCRIPT_NAME version $SCRIPT_VERSION"; }

file_size() {
    # Portable file size (macOS uses -f%z, GNU/Linux uses -c%s).
    stat -f%z "$1" 2>/dev/null || stat -c%s "$1" 2>/dev/null || echo "0"
}

# -----------------------------------------------------------------------------
# Font installation
# -----------------------------------------------------------------------------
install_fonts() {
    local temp_dir="$1"
    local installed_count=0

    for target in "${FONT_FILES[@]}"; do
        # Match the exact archive filename first; fall back to a loose match.
        local source_file
        source_file=$(find "$temp_dir" -type f -name "$target" | head -1)
        if [[ -z "$source_file" ]]; then
            source_file=$(find "$temp_dir" -type f -name "*${target}" | head -1)
        fi

        if [[ -n "$source_file" && -f "$source_file" ]]; then
            cp "$source_file" "$FONT_DIR/$target"
            log_success "Installed $target"
            installed_count=$((installed_count + 1))
        else
            log_warning "Could not find '$target' in the downloaded archive"
        fi
    done

    echo "$installed_count"
}

verify_installation() {
    log_info "Verifying installation..."
    # The engine only strictly requires a Regular variant (Mono preferred).
    local required=(
        "JetBrainsMonoNerdFontMono-Regular.ttf"
        "JetBrainsMonoNerdFont-Regular.ttf"
    )
    local have_regular=false
    for f in "${required[@]}"; do
        local path="$FONT_DIR/$f"
        if [[ -f "$path" && "$(file_size "$path")" -gt 1000 ]]; then
            log_success "$f ($(( $(file_size "$path") / 1024 )) KB)"
            have_regular=true
        fi
    done

    if [[ "$have_regular" == "true" ]]; then
        log_success "A usable Regular font variant is installed"
        return 0
    fi
    log_error "No usable Regular font variant found in $FONT_DIR"
    return 1
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------
main() {
    local clean_install=false
    local debug_mode=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)    print_usage; exit 0 ;;
            -v|--version) print_version; exit 0 ;;
            -d|--debug)   debug_mode=true; DEBUG=true; shift ;;
            --clean)      clean_install=true; shift ;;
            *)            log_error "Unknown option: $1"; print_usage; exit 1 ;;
        esac
    done

    trap cleanup EXIT

    log_info "Starting font installation..."
    log_debug "Font directory: $FONT_DIR"
    log_debug "Download URL: $DOWNLOAD_URL"

    mkdir -p "$FONT_DIR"

    if [[ "$clean_install" == "true" ]]; then
        log_info "Cleaning existing fonts in $FONT_DIR..."
        rm -f "$FONT_DIR"/*.ttf 2>/dev/null || true
    fi

    TEMP_DIR=$(mktemp -d)
    log_debug "Created temporary directory: $TEMP_DIR"

    log_info "Downloading JetBrains Mono Nerd Font..."
    if ! curl -L -o "$TEMP_DIR/$ZIP_FILE" "$DOWNLOAD_URL" --fail --progress-bar; then
        log_error "Failed to download font archive"
        log_info "Trying fallback to version 3.3.0..."
        local fallback_url="https://github.com/ryanoasis/nerd-fonts/releases/download/v3.3.0/$ZIP_FILE"
        if ! curl -L -o "$TEMP_DIR/$ZIP_FILE" "$fallback_url" --fail --progress-bar; then
            log_error "Fallback download also failed"
            log_info "Please download manually from: https://github.com/ryanoasis/nerd-fonts/releases"
            exit 1
        fi
    fi

    if [[ ! -s "$TEMP_DIR/$ZIP_FILE" ]]; then
        log_error "Downloaded file is empty or corrupted"
        exit 1
    fi
    log_success "Download completed"

    log_info "Extracting font files..."
    if ! unzip -q "$TEMP_DIR/$ZIP_FILE" -d "$TEMP_DIR/extracted"; then
        log_error "Failed to extract font archive"
        exit 1
    fi
    log_success "Extraction completed"

    if [[ "$debug_mode" == "true" ]]; then
        log_info "Found font files:"
        find "$TEMP_DIR/extracted" -name "*.ttf" -type f | head -10 | while read -r font_file; do
            echo "  - $(basename "$font_file")"
        done
    fi

    local installed_count
    installed_count=$(install_fonts "$TEMP_DIR/extracted")

    if verify_installation; then
        log_success "Font installation completed successfully"
        log_info "Installed $installed_count font file(s) into: $FONT_DIR"
        ls -lh "$FONT_DIR"/*.ttf 2>/dev/null || log_warning "No font files found"
    else
        log_warning "Font installation completed with warnings"
        exit 1
    fi
}

# -----------------------------------------------------------------------------
# Entry point
# -----------------------------------------------------------------------------
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
