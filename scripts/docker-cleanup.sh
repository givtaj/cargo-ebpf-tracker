#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"

dry_run=0
assume_yes=0
full_prune=0

compose_files=()
cache_roots=()

usage() {
  cat <<'EOF'
Usage: bash scripts/docker-cleanup.sh [--dry-run] [--yes] [--all]

Default cleanup:
- stops and removes this repo's Docker Compose stacks
- stops and removes cached tracker and Jaeger Compose runtimes under the tracker cache dir
- only touches the compose project derived from each tracked compose file
- leaves global Docker cache alone unless `--all` is passed

Options:
  --all      run a full Docker prune after the tracker-specific cleanup
  --dry-run  print the commands without executing them
  --yes      skip the confirmation prompt
  -h, --help show this help text
EOF
}

append_unique() {
  local candidate="$1"
  local existing

  [ -n "${candidate}" ] || return 0

  for existing in "${@:2}"; do
    if [ "${existing}" = "${candidate}" ]; then
      return 0
    fi
  done

  return 1
}

add_cache_root() {
  local candidate="$1"

  if append_unique "${candidate}" "${cache_roots[@]:-}"; then
    return 0
  fi

  cache_roots+=("${candidate}")
}

add_compose_file() {
  local candidate="$1"

  [ -f "${candidate}" ] || return 0

  if append_unique "${candidate}" "${compose_files[@]:-}"; then
    return 0
  fi

  compose_files+=("${candidate}")
}

compose_project_name_for_file() {
  local compose_file="$1"
  local project_dir
  local project_name

  project_dir="$(dirname -- "${compose_file}")"
  project_name="$(basename -- "${project_dir}")"
  project_name="$(printf '%s' "${project_name}" | tr '[:upper:]' '[:lower:]')"
  project_name="${project_name//[^a-z0-9]/-}"

  if [ -z "${project_name}" ]; then
    project_name="ebpf-tracker"
  fi

  printf '%s' "${project_name}"
}

print_command() {
  local part

  printf "+"
  for part in "$@"; do
    printf " %q" "${part}"
  done
  printf "\n"
}

run_cmd() {
  if [ "${dry_run}" -eq 1 ]; then
    print_command "$@"
    return 0
  fi

  "$@"
}

run_compose_down() {
  local compose_file="$1"
  local project_name="$2"

  if [ "${dry_run}" -eq 1 ]; then
    print_command PROJECT_DIR="${repo_root}" docker compose -p "${project_name}" -f "${compose_file}" down --remove-orphans --rmi all --volumes
    return 0
  fi

  PROJECT_DIR="${repo_root}" docker compose -p "${project_name}" -f "${compose_file}" down --remove-orphans --rmi all --volumes
}

collect_cache_roots() {
  if [ -n "${EBPF_TRACKER_CACHE_DIR:-}" ]; then
    add_cache_root "${EBPF_TRACKER_CACHE_DIR}"
    return 0
  fi

  if [ -n "${XDG_CACHE_HOME:-}" ]; then
    add_cache_root "${XDG_CACHE_HOME}/ebpf-tracker"
  fi

  if [ -n "${HOME:-}" ]; then
    add_cache_root "${HOME}/.cache/ebpf-tracker"
  fi
}

collect_compose_files() {
  local root
  local compose_file

  add_compose_file "${repo_root}/docker-compose.bpftrace.yml"
  add_compose_file "${repo_root}/docker-compose.bpftrace.node.yml"
  add_compose_file "${repo_root}/crates/ebpf-tracker-otel/docker-compose.jaeger.yml"

  collect_cache_roots

  for root in "${cache_roots[@]:-}"; do
    [ -d "${root}" ] || continue

    while IFS= read -r compose_file; do
      [ -n "${compose_file}" ] || continue
      add_compose_file "${compose_file}"
    done < <(find "${root}" -type f \( -name 'docker-compose*.yml' -o -name 'docker-compose*.yaml' \) -print 2>/dev/null)
  done
}

confirm() {
  local reply

  if [ "${assume_yes}" -eq 1 ] || [ "${dry_run}" -eq 1 ]; then
    return 0
  fi

  printf "Continue? [y/N] "
  read -r reply

  case "${reply}" in
    y|Y|yes|YES)
      ;;
    *)
      echo "Aborted."
      exit 0
      ;;
  esac
}

show_scope() {
  local compose_file
  local project_name

  echo "Docker cleanup scope:"
  echo "- repo root: ${repo_root}"

  if [ "${#compose_files[@]}" -gt 0 ]; then
    echo "- compose stacks:"
    for compose_file in "${compose_files[@]}"; do
      project_name="$(compose_project_name_for_file "${compose_file}")"
      echo "  ${compose_file} (project: ${project_name})"
    done
  else
    echo "- compose stacks: none found"
  fi

  if [ "${full_prune}" -eq 1 ]; then
    echo "- extra prune: docker system prune -a --volumes"
  else
    echo "- extra prune: none"
  fi
}

cleanup_compose_stacks() {
  local compose_file
  local project_name

  for compose_file in "${compose_files[@]}"; do
    project_name="$(compose_project_name_for_file "${compose_file}")"
    echo
    echo "Cleaning Compose stack: ${compose_file} (project: ${project_name})"
    if ! run_compose_down "${compose_file}" "${project_name}"; then
      echo "warning: failed to clean ${compose_file}" >&2
    fi
  done
}

show_disk_usage() {
  if [ "${dry_run}" -eq 1 ]; then
    return 0
  fi

  echo
  echo "Docker disk usage:"
  docker system df
}

parse_args() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --all)
        full_prune=1
        ;;
      --dry-run)
        dry_run=1
        ;;
      --yes)
        assume_yes=1
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "unknown option: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
    shift
  done
}

main() {
  parse_args "$@"

  if ! command -v docker >/dev/null 2>&1; then
    echo "docker is required for this cleanup script" >&2
    exit 1
  fi

  if [ "${dry_run}" -eq 0 ]; then
    docker info >/dev/null 2>&1 || {
      echo "docker daemon is not available; start Docker and retry" >&2
      exit 1
    }
  fi

  collect_compose_files
  show_scope
  show_disk_usage
  echo
  echo "This will stop/remove tracker Docker resources."
  confirm

  cleanup_compose_stacks

  if [ "${full_prune}" -eq 1 ]; then
    echo
    echo "Running full Docker system prune."
    run_cmd docker system prune -a --force --volumes
  fi

  show_disk_usage
}

main "$@"
