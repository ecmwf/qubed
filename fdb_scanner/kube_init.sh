#!/usr/bin/env bash

set -euo pipefail

script_name=$(basename "$0")

usage() {
	cat <<EOF
Usage: $script_name <source_directory> [destination_path] [namespace]

Copy the contents of <source_directory> into the shared storage mounted by the
stac-server pod. The optional destination path defaults to /data/shared, and
the namespace defaults to the namespace of the current Kubernetes context (or
"default" if unset).
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
	usage
	exit 0
fi

if [[ $# -lt 1 ]]; then
	echo "Error: source directory is required." >&2
	echo >&2
	usage >&2
	exit 1
fi

if ! command -v kubectl >/dev/null 2>&1; then
	echo "Error: kubectl is not installed or not in PATH." >&2
	exit 1
fi

source_dir=$1
destination_path=${2:-/data/shared}

current_namespace() {
	kubectl config view --minify --output 'jsonpath={..namespace}' 2>/dev/null | awk 'NF'
}

namespace=${3:-$(current_namespace)}

if [[ -z "$namespace" ]]; then
	namespace=default
fi

if [[ ! -d "$source_dir" ]]; then
	echo "Error: source directory '$source_dir' does not exist." >&2
	exit 1
fi

if ! source_dir_abs=$(cd "$source_dir" 2>/dev/null && pwd); then
	echo "Error: unable to resolve absolute path for '$source_dir'." >&2
	exit 1
fi

readarray -t stac_pods < <(kubectl get pods -n "$namespace" -l app=stac-server \
	-o jsonpath='{range .items[?(@.status.phase=="Running")]}{.metadata.name}{"\n"}{end}')

if [[ ${#stac_pods[@]} -eq 0 || -z "${stac_pods[0]:-}" ]]; then
	echo "Error: no running stac-server pods found in namespace '$namespace'." >&2
	exit 1
fi

target_pod=${stac_pods[0]}

echo "Uploading contents of '$source_dir_abs' to pod '$target_pod' in namespace '$namespace'."
echo "Destination path: $destination_path"

shopt -s dotglob nullglob
entries=("$source_dir_abs"/*)

if [[ ${#entries[@]} -eq 0 ]]; then
	echo "Warning: source directory is empty. Nothing to upload."
fi

for entry in "${entries[@]}"; do
	name=$(basename "$entry")
	target="$destination_path/$name"
	if [[ -d "$entry" ]]; then
		echo "Copying directory '$name'"
		kubectl exec "$target_pod" -n "$namespace" -- mkdir -p "$target"
		kubectl cp "$entry/." "$target_pod:$target" -n "$namespace"
	else
		echo "Copying file '$name'"
		kubectl cp "$entry" "$target_pod:$target" -n "$namespace"
	fi
done

echo "Upload complete."
