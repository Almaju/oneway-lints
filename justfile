oneway:
    #!/usr/bin/env bash
    set -euo pipefail
    export ONEWAY_LINTS_PATH="$PWD/lints"
    for crate in cli lints; do
        echo "==> cargo oneway in $crate/"
        (cd "$crate" && cargo oneway)
    done
