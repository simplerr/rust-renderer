#!/usr/bin/env bash
# Run rust-renderer on Apple GPUs via the KosmicKrisp (Mesa) Vulkan-on-Metal driver.
#
# Prerequisites (installed via Homebrew): vulkan-loader, shaderc, plus a locally
# built Mesa KosmicKrisp driver at $MESA_BUILD below. See the plan for build steps.
set -euo pipefail

MESA_BUILD="${MESA_BUILD:-$HOME/git/mesa/builddir}"
ICD="$MESA_BUILD/src/kosmickrisp/vulkan/kosmickrisp_mesa_devenv_icd.aarch64.json"

if [[ ! -f "$ICD" ]]; then
  echo "KosmicKrisp ICD not found at: $ICD" >&2
  echo "Build Mesa with -Dvulkan-drivers=kosmickrisp first." >&2
  exit 1
fi

# Loader: point it at the KosmicKrisp ICD (no RT support -> renderer auto-selects a
# rasterized graph mode). VK_LOADER_DEBUG=error keeps loader noise down; bump to
# 'all' to debug ICD selection.
export VK_ICD_FILENAMES="$ICD"
export VK_LOADER_DEBUG="${VK_LOADER_DEBUG:-error}"

# Build-time: libvulkan / libshaderc live in the Homebrew prefix.
export RUSTFLAGS="${RUSTFLAGS:--L /opt/homebrew/lib}"
export SHADERC_LIB_DIR="${SHADERC_LIB_DIR:-/opt/homebrew/lib}"

exec cargo run -p prototype "$@"
