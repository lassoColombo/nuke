# Seed a kind cluster with the objects the fixture corpus is captured from.
#
#     nu tests/fixtures/seed/seed.nu kind-bench-k8s04 crds
#     nu tests/fixtures/seed/seed.nu kind-bench-console builtin
#
# `<dir>/crds.txt`, when present, lists upstream CRD manifests to install
# first (pinned URLs, one per line). Every `<dir>/*.yaml` is then applied
# twice: once as the object, once against its status subresource, so a
# status no controller will ever write still lands where a CRD keeps it.
# Server-side apply, so the large CRDs fit and reruns converge.
#
# Only ever point this at a kind cluster.
def main [context: string, dir: string] {
    let base = ($env.CURRENT_FILE | path dirname | path join $dir)
    let kubectl = [kubectl --context $context]

    if not ($context | str starts-with "kind-") {
        error make { msg: $"refusing to seed ($context): not a kind context" }
    }

    let crds = ($base | path join crds.txt)
    if ($crds | path exists) {
        for url in (open $crds | lines | where { |l| ($l | str trim | is-not-empty) and not ($l | str starts-with "#") }) {
            print $"crd ($url)"
            ^kubectl --context $context apply --server-side --force-conflicts -f $url | ignore
        }
        ^kubectl --context $context wait --for condition=established crd --all --timeout=120s | ignore
    }

    for file in (glob ($base | path join "*.yaml") | sort) {
        print $"apply ($file | path basename)"
        ^kubectl --context $context apply --server-side --force-conflicts -f $file | ignore
        # A resource without a status subresource (or without a status in
        # the file) rejects this; the first apply already carried it.
        try {
            ^kubectl --context $context apply --server-side --force-conflicts --subresource=status -f $file o+e> /dev/null
        }
    }

    # A CSR labelled for approval: the apiserver's signer issues it.
    let csrs = (^kubectl --context $context get csr -l seed.nuke/approve=true -o name | lines)
    for csr in $csrs {
        ^kubectl --context $context certificate approve $csr | ignore
    }
}
