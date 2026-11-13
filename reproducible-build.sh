#!/usr/bin/env bash
# Reproducible (Docker) build of selected contracts, with the image pinned in
# sc-reproducible-build.toml. The release workflow (.github/workflows/release.yml)
# runs this script, so a local run gives the same codehashes as the release.
#
# Only committed files are built: the source is a `git archive` export of the
# commit, without local changes or untracked files.
#
# Usage:
#   ./reproducible-build.sh [options] <contract>...
#
# Contracts are package names from Cargo.toml (pair, router, farm-staking, ...),
# separated by spaces or commas.
#
# Options:
#   --ref <git-ref>   commit, tag or branch to build (default: HEAD)
#   --output <dir>    output folder (default: target/reproducible-build)
#   --image <image>   Docker image, instead of the one in sc-reproducible-build.toml
#   --list            print the contract names that can be built, then exit
#   -h, --help        print this help
#
# Output:
#   <output>/<contract>/      sc-meta output (artifacts.json, .wasm, .abi.json, .source.json, ...)
#   <output>/release-assets/  files attached to the GitHub release
#   <output>/codehashes.md    Docker image, commit and blake2b codehashes
#
# Examples:
#   ./reproducible-build.sh pair router
#   ./reproducible-build.sh --ref v3.5.0 farm-staking    # check the codehashes of a release

set -euo pipefail

readonly MIN_SC_META_VERSION="0.66.1"
readonly CONFIG_FILE="sc-reproducible-build.toml"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT
readonly NOTES_SCRIPT="$REPO_ROOT/.github/scripts/release_notes.py"
readonly OUTPUT_MARKER=".reproducible-build"

die() {
    echo "error: $*" >&2
    exit 1
}

usage() {
    awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"
}

# Prints "<name> <folder>" for every contract (folder with multiversx.json) in $1.
list_contracts() {
    local root="$1" file dir name
    (cd "$root" && find . -name multiversx.json -not -path '*/target/*' | sort) | while IFS= read -r file; do
        dir="$(dirname "${file#./}")"
        name="$(sed -n 's/^name *= *"\(.*\)"/\1/p' "$root/$dir/Cargo.toml" | head -n 1)"
        echo "$name $dir"
    done
}

# Prints the docker-image value from the config file $1, if any.
config_image() {
    [[ -f "$1" ]] || return 0
    sed -n 's/^docker-image *= *"\(.*\)"/\1/p' "$1" | head -n 1
}

version_at_least() {
    [[ "$(printf '%s\n%s\n' "$2" "$1" | sort -V | head -n 1)" == "$2" ]]
}

# --- arguments ---------------------------------------------------------------

ref="HEAD"
output="$REPO_ROOT/target/reproducible-build"
image=""
list_only=false
contracts=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --ref)    [[ $# -ge 2 ]] || die "--ref needs a value"; ref="$2"; shift 2 ;;
        --output) [[ $# -ge 2 ]] || die "--output needs a value"; output="$2"; shift 2 ;;
        --image)  [[ $# -ge 2 ]] || die "--image needs a value"; image="$2"; shift 2 ;;
        --list)   list_only=true; shift ;;
        -h|--help) usage; exit 0 ;;
        -*) die "unknown option: $1 (see --help)" ;;
        *)
            IFS=$', \t\n' read -r -a names <<< "$1"
            for name in ${names[@]+"${names[@]}"}; do
                [[ -n "$name" ]] || continue
                case " ${contracts[*]-} " in
                    *" $name "*) ;;
                    *) contracts+=("$name") ;;
                esac
            done
            shift
            ;;
    esac
done

# --- source ------------------------------------------------------------------

cd "$REPO_ROOT"
commit="$(git rev-parse --verify --quiet "${ref}^{commit}" || git rev-parse --verify --quiet "origin/${ref}^{commit}")" \
    || die "unknown git ref: $ref"

src="$(mktemp -d "${TMPDIR:-/tmp}/mx-exchange-rb-src.XXXXXX")"
trap 'rm -rf "$src"' EXIT
git archive --format=tar "$commit" | tar -xf - -C "$src"
available="$(list_contracts "$src")"

if $list_only; then
    echo "Contracts at $commit:"
    echo "$available" | awk '{ printf "  %-28s %s\n", $1, $2 }'
    exit 0
fi

[[ ${#contracts[@]} -gt 0 ]] || die "no contract given (see --help, or --list for the names)"
for name in "${contracts[@]}"; do
    awk -v name="$name" '$1 == name { found = 1 } END { exit !found }' <<< "$available" \
        || die "unknown contract '$name' at $commit. Run with --list to see the names."
done

if [[ -z "$image" ]]; then
    # The config of the built commit wins; older commits don't have one.
    image="$(config_image "$src/$CONFIG_FILE")"
    [[ -n "$image" ]] || image="$(config_image "$REPO_ROOT/$CONFIG_FILE")"
    [[ -n "$image" ]] || die "no docker-image in $CONFIG_FILE; pass --image"
fi

# --- tools -------------------------------------------------------------------

command -v python3 >/dev/null || die "python3 not found"
command -v docker >/dev/null || die "docker not found"
docker info >/dev/null 2>&1 || die "the Docker daemon is not running"
command -v sc-meta >/dev/null \
    || die "sc-meta not found. Install it: cargo install multiversx-sc-meta --version $MIN_SC_META_VERSION --locked"
sc_meta_version="$(sc-meta --version | awk '{ print $2 }')"
version_at_least "$sc_meta_version" "$MIN_SC_META_VERSION" \
    || die "sc-meta $sc_meta_version is too old, need >= $MIN_SC_META_VERSION"

# --- output folder -----------------------------------------------------------

mkdir -p "$output"
output="$(cd "$output" && pwd)"
if [[ -n "$(ls -A "$output")" ]]; then
    [[ -f "$output/$OUTPUT_MARKER" ]] \
        || die "output folder is not empty and was not made by this script: $output"
    rm -rf "$output"
    mkdir -p "$output"
fi
touch "$output/$OUTPUT_MARKER"

if [[ "$ref" == "HEAD" && -n "$(git status --porcelain --untracked-files=no)" ]]; then
    echo "warning: local changes are not built, only commit $commit" >&2
fi

echo "Commit:    $commit ($ref)"
echo "Image:     $image"
echo "Contracts: ${contracts[*]}"
echo "Output:    $output"

# --- build -------------------------------------------------------------------

for name in "${contracts[@]}"; do
    echo
    echo "==> Building $name"
    sc-meta reproducible-build build \
        --project "$src" \
        --docker-image "$image" \
        --contract "$name" \
        --output "$output/$name" \
        --no-docker-tty \
        --no-docker-interactive
done

# --- release assets and codehashes ------------------------------------------

assets="$output/release-assets"
mkdir -p "$assets"
for name in "${contracts[@]}"; do
    [[ -f "$output/$name/artifacts.json" ]] || die "$name: no artifacts.json in $output/$name"
    cp "$output/$name/artifacts.json" "$assets/$name.artifacts.json"
    while IFS= read -r -d '' file; do
        [[ ! -e "$assets/$(basename "$file")" ]] || die "two contracts produce $(basename "$file")"
        cp "$file" "$assets/"
    done < <(find "$output/$name" -mindepth 2 -type f \
        \( -name '*.wasm' -o -name '*.abi.json' -o -name '*.source.json' \) -print0)
done

python3 "$NOTES_SCRIPT" summary \
    --assets "$assets" \
    --image "$image" \
    --commit "$commit" \
    --output "$output/codehashes.md"

echo
cat "$output/codehashes.md"
echo
echo "Done. Release files: $assets"
