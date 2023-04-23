[no-exit-message]
package:
	#!/usr/bin/env bash
	set -euo pipefail
	WORKDIR="$PWD"
	mkdir -p "$WORKDIR/bin"
	vsce package --out bin/
