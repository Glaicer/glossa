#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
readonly TARGET_DIR="${SCRIPT_DIR}/target/release"
readonly GLOSSA_ASSET_NAME="glossa-linux-x86_64.tar.gz"
readonly LEGACY_DOTOOL_ASSET_NAME="dotool-linux-x86_64.tar.gz"
readonly CHECKSUMS_NAME="sha256sums.txt"
readonly TTY_FD_DEFAULT=3

tty_fd="${TTY_FD_DEFAULT}"
release_version=""
staging_dir=""

cleanup() {
  if [[ -n "${staging_dir}" && -d "${staging_dir}" ]]; then
    rm -rf "${staging_dir}"
  fi
}
trap cleanup EXIT

log() {
  printf '%s\n' "$*"
}

die() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}

open_tty() {
  [[ -r /dev/tty ]] || die "An interactive TTY is required to run this packaging script."
  exec {tty_fd}<> /dev/tty
}

prompt() {
  local message="$1"
  printf '%s' "${message}" >&${tty_fd}
}

read_tty() {
  local answer
  IFS= read -r answer <&${tty_fd} || true
  printf '%s' "${answer}"
}

normalize_answer() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]'
}

confirm_continue() {
  log "Glossa release packager"
  log
  log "This script will:"
  log "- build the release Glossa binary with cargo"
  log "- assemble the single archive layout expected by install.sh"
  log "- write ${TARGET_DIR}/${GLOSSA_ASSET_NAME}"
  log "- refresh ${TARGET_DIR}/${CHECKSUMS_NAME}"
  log

  while true; do
    prompt "Continue with release packaging? [Y/n] "
    case "$(normalize_answer "$(read_tty)")" in
      "" | y | yes)
        return 0
        ;;
      n | no)
        log "Packaging cancelled."
        exit 0
        ;;
      *)
        log "Please answer yes or no."
        ;;
    esac
  done
}

prompt_release_version() {
  while true; do
    prompt "Release version: "
    release_version="$(read_tty)"

    if [[ -n "${release_version}" ]]; then
      return 0
    fi

    log "The release version cannot be empty."
  done
}

require_command() {
  local command_name="$1"
  command -v "${command_name}" >/dev/null 2>&1 || die "Required command '${command_name}' is not available."
}

assert_prerequisites() {
  require_command cargo
  require_command tar
  require_command sha256sum
  require_command mktemp
}

resolve_dotool_payload() {
  local file_name="$1"
  local candidate

  for candidate in \
    "${SCRIPT_DIR}/contrib/dotool/binary/${file_name}" \
    "${SCRIPT_DIR}/contrib/dotool/source/dotool/${file_name}"
  do
    if [[ -f "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  die "Could not find required dotool payload file '${file_name}' in contrib/dotool/binary or contrib/dotool/source/dotool."
}

build_release_binary() {
  log "Building Glossa in release mode"
  cargo build --release --package glossa-bin
}

remove_legacy_assets() {
  if [[ -f "${TARGET_DIR}/${LEGACY_DOTOOL_ASSET_NAME}" ]]; then
    rm -f "${TARGET_DIR}/${LEGACY_DOTOOL_ASSET_NAME}"
  fi
}

stage_release_contents() {
  local archive_root="${staging_dir}"
  local dotool_root="${archive_root}/dotool"
  local dotool_path
  local dotoolc_path
  local dotoold_path
  local rules_path

  mkdir -p "${archive_root}/assets/tray" "${archive_root}/assets/sounds" "${dotool_root}"
  install -m755 "${TARGET_DIR}/glossa" "${archive_root}/glossa"
  install -m644 "${SCRIPT_DIR}/contrib/assets/tray/"* "${archive_root}/assets/tray/"
  install -m644 "${SCRIPT_DIR}/contrib/assets/sounds/"* "${archive_root}/assets/sounds/"
  dotool_path="$(resolve_dotool_payload "dotool")"
  dotoolc_path="$(resolve_dotool_payload "dotoolc")"
  dotoold_path="$(resolve_dotool_payload "dotoold")"
  rules_path="$(resolve_dotool_payload "80-dotool.rules")"

  install -m755 "${dotool_path}" "${dotool_root}/dotool"
  install -m755 "${dotoolc_path}" "${dotool_root}/dotoolc"
  install -m755 "${dotoold_path}" "${dotool_root}/dotoold"
  install -m644 "${rules_path}" "${dotool_root}/80-dotool.rules"
  printf '%s\n' "${release_version}" > "${archive_root}/VERSION"
}

write_archive() {
  log "Creating ${GLOSSA_ASSET_NAME}"
  tar -czf "${TARGET_DIR}/${GLOSSA_ASSET_NAME}" \
    -C "${staging_dir}" \
    assets \
    dotool \
    glossa \
    VERSION
}

write_checksums() {
  log "Writing ${CHECKSUMS_NAME}"
  (
    cd "${TARGET_DIR}"
    sha256sum "${GLOSSA_ASSET_NAME}" > "${CHECKSUMS_NAME}"
  )
}

print_summary() {
  log
  log "Release packaging complete."
  log "- Version: ${release_version}"
  log "- Tarball: ${TARGET_DIR}/${GLOSSA_ASSET_NAME}"
  log "- Checksums: ${TARGET_DIR}/${CHECKSUMS_NAME}"
}

main() {
  open_tty
  confirm_continue
  prompt_release_version
  assert_prerequisites
  mkdir -p "${TARGET_DIR}"
  staging_dir="$(mktemp -d)"
  build_release_binary
  remove_legacy_assets
  stage_release_contents
  write_archive
  write_checksums
  print_summary
}

if [[ -n "${BASH_SOURCE[0]:-}" && "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi
