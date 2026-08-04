#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
version_file="${repo_root}/OPEN_GROK_VERSION"
dist_dir="${repo_root}/dist"
artifact_name="open-grok-ubuntu-20.04-x86_64"
archive_name="${artifact_name}.tar.gz"
target_triple="x86_64-unknown-linux-gnu"
expected_rg_version="ripgrep 15.0.0"
artifact_path="${dist_dir}/${artifact_name}"
checksum_path="${artifact_path}.sha256"
archive_path="${dist_dir}/${archive_name}"
archive_checksum_path="${archive_path}.sha256"

if [[ ! -f "$version_file" ]]; then
    echo "Error: missing $version_file" >&2
    exit 1
fi

version="$(sed -n '1p' "$version_file" | tr -d '\r')"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?$ ]]; then
    echo "Error: invalid Open Grok version '$version' in $version_file" >&2
    exit 1
fi

if [[ "$(uname -s)" != "Linux" ]] || [[ "$(uname -m)" != "x86_64" ]]; then
    echo "Error: this release builder requires x86_64 Linux." >&2
    exit 1
fi

for command in cargo file git readelf sha256sum strip tar; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "Error: required command not found: $command" >&2
        exit 1
    fi
done

protoc_path="${PROTOC:-}"
if [[ -z "$protoc_path" || ! -f "$protoc_path" || ! -x "$protoc_path" ]]; then
    echo "Error: set PROTOC to a verified protoc 29.3 executable." >&2
    exit 1
fi
if [[ "$("$protoc_path" --version)" != "libprotoc 29.3" ]]; then
    echo "Error: release builds require libprotoc 29.3 at PROTOC=$protoc_path." >&2
    exit 1
fi

if [[ -n "$(git -C "$repo_root" status --porcelain --untracked-files=normal)" ]]; then
    echo "Error: release builds require a clean git worktree." >&2
    echo "Commit or remove all tracked and untracked changes, then retry." >&2
    exit 1
fi
commit="$(git -C "$repo_root" rev-parse --short HEAD)"

rg_path="${GROK_TOOLS_BUNDLE_RG_PATH:-}"
if [[ -z "$rg_path" || ! -f "$rg_path" || ! -x "$rg_path" ]]; then
    echo "Error: set GROK_TOOLS_BUNDLE_RG_PATH to a verified x86_64 ripgrep executable." >&2
    exit 1
fi
rg_path="$(cd "$(dirname "$rg_path")" && pwd)/$(basename "$rg_path")"
if ! file "$rg_path" | grep -Eq 'x86-64|x86_64'; then
    echo "Error: the bundled ripgrep executable is not x86_64: $rg_path" >&2
    exit 1
fi
rg_version_line="$("$rg_path" --version | sed -n '1p')"
rg_version="$(printf '%s\n' "$rg_version_line" | awk '{ print $1 " " $2 }')"
if [[ "$rg_version" != "$expected_rg_version" ]]; then
    echo "Error: release builds require ${expected_rg_version}; found '${rg_version_line}'." >&2
    exit 1
fi

mkdir -p "$dist_dir"
staged_artifact="${dist_dir}/.${artifact_name}.tmp.$$"
staged_checksum="${dist_dir}/.${artifact_name}.sha256.tmp.$$"
staged_archive="${dist_dir}/.${archive_name}.tmp.$$"
staged_archive_checksum="${dist_dir}/.${archive_name}.sha256.tmp.$$"
cleanup() {
    rm -f \
        "$staged_artifact" \
        "$staged_checksum" \
        "$staged_archive" \
        "$staged_archive_checksum"
}
trap cleanup EXIT

echo "Refreshing version/commit build metadata..." >&2
cd "$repo_root"
cargo clean \
    --quiet \
    --profile release-dist \
    --target "$target_triple" \
    -p xai-grok-pager-bin \
    -p xai-grok-pager \
    -p xai-grok-tools

echo "Building Open Grok ${version} (${commit}) for Ubuntu 20.04..." >&2
GROK_VERSION="$version" \
    GROK_TOOLS_BUNDLE_RG_PATH="$rg_path" \
    CARGO_INCREMENTAL=0 \
    cargo build \
    --locked \
    --profile release-dist \
    --features release-dist \
    --target "$target_triple" \
    -p xai-grok-pager-bin \
    --bin open-grok

source_binary="${repo_root}/target/${target_triple}/release-dist/open-grok"
if [[ ! -x "$source_binary" ]]; then
    echo "Error: Cargo did not produce $source_binary" >&2
    exit 1
fi

cp "$source_binary" "$staged_artifact"
chmod 0755 "$staged_artifact"
strip "$staged_artifact"

if ! file "$staged_artifact" | grep -Eq 'ELF 64-bit.*x86-64'; then
    echo "Error: release artifact is not an x86_64 ELF binary." >&2
    exit 1
fi

version_output="$($staged_artifact --version)"
if [[ "$version_output" != *"$version"* ]]; then
    echo "Error: release version verification failed: $version_output" >&2
    exit 1
fi
if [[ "$version_output" != *"$commit"* ]]; then
    echo "Error: release commit verification failed: $version_output" >&2
    exit 1
fi

max_glibc="$({ readelf --version-info "$staged_artifact" || true; } \
    | grep -oE 'GLIBC_[0-9]+(\.[0-9]+)+' \
    | sed 's/GLIBC_//' \
    | sort -Vu \
    | tail -n 1)"
if [[ -z "$max_glibc" ]]; then
    echo "Error: could not determine the artifact's maximum GLIBC requirement." >&2
    exit 1
fi
if [[ "$(printf '%s\n' "$max_glibc" "2.31" | sort -V | tail -n 1)" != "2.31" ]]; then
    echo "Error: artifact requires GLIBC_${max_glibc}, newer than Ubuntu 20.04's GLIBC_2.31." >&2
    exit 1
fi

checksum="$(sha256sum "$staged_artifact" | awk '{ print $1 }')"
printf '%s  %s\n' "$checksum" "$artifact_name" > "$staged_checksum"
tar_root="$(mktemp -d)"
trap 'rm -rf "$tar_root"; cleanup' EXIT
cp "$staged_artifact" "$tar_root/$artifact_name"
tar -C "$tar_root" -czf "$staged_archive" "$artifact_name"
rm -rf "$tar_root"
archive_checksum="$(sha256sum "$staged_archive" | awk '{ print $1 }')"
printf '%s  %s\n' "$archive_checksum" "$archive_name" > "$staged_archive_checksum"

mv -f "$staged_artifact" "$artifact_path"
mv -f "$staged_checksum" "$checksum_path"
mv -f "$staged_archive" "$archive_path"
mv -f "$staged_archive_checksum" "$archive_checksum_path"
trap - EXIT

echo "Release assets:" >&2
echo "  $artifact_path" >&2
echo "  $checksum_path" >&2
echo "  $archive_path" >&2
echo "  $archive_checksum_path" >&2
