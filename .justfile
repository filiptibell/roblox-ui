[no-exit-message]
build:
	#!/usr/bin/env bash
	set -euo pipefail
	WORKDIR="$PWD"
	rm -rf "$WORKDIR/out"
	rm -rf "$WORKDIR/bin"
	mkdir -p "$WORKDIR/bin"
	echo "🛠️  Building extension..."
	vsce package --out "$WORKDIR/bin/" > /dev/null

[no-exit-message]
install: build
	#!/usr/bin/env bash
	set -euo pipefail
	WORKDIR="$PWD"
	EXTENSION=$(find "$WORKDIR/bin/" -name "*.vsix")
	echo "🚀 Installing extension..."
	code --install-extension "$EXTENSION" > /dev/null
	echo "✅ Installed extension successfully!"

[no-exit-message]
publish-extension:
	#!/usr/bin/env bash
	set -euo pipefail
	echo "🛠️  Publishing extension..."
	vsce publish > /dev/null
	echo "✅ Published extension successfully!"
