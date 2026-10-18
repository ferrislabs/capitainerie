#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cluster="capitainerie"
namespace="${EXECUTOR_NAMESPACE:-capitainerie-tasks}"
image="${EXECUTOR_IMAGE:-capitainerie-executor:dev}"

for tool in kind kubectl docker; do
  command -v "$tool" >/dev/null || { echo "$tool is required" >&2; exit 1; }
done

if ! kind get clusters | grep -qx "$cluster"; then
  kind create cluster --config "$root/deploy/kind/cluster.yaml"
fi

context="kind-$cluster"
kubectl --context "$context" create namespace "$namespace" --dry-run=client -o yaml \
  | kubectl --context "$context" apply -f -

docker build -t "$image" -f "$root/deploy/executor/Dockerfile" "$root/deploy/executor"
kind load docker-image "$image" --name "$cluster"

echo "kubeconfig context: $context"
echo "tasks namespace:    $namespace"
echo "executor image:     $image"
