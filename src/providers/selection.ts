import * as vscode from "vscode";

import { RojoTreeProvider } from "./tree";

export class SelectionProvider implements vscode.Disposable {
	private disposables: Array<vscode.Disposable> = new Array();

	constructor(
		treeView: vscode.TreeView<vscode.TreeItem>,
		treeDataProvider: RojoTreeProvider
	) {
		this.disposables.push(
			treeView.onDidChangeSelection((event) => {
				const selected: Array<any> = Array.from(event.selection);

				let canMove = false;
				let canPaste = false;
				let canPasteInto = false;
				try {
					canMove = selected.every((item) => item.canMove());
					canPaste = selected.every((item) => item.canPaste());
					canPasteInto = selected.every((item) =>
						item.canPasteInto()
					);
				} catch {}

				vscode.commands.executeCommand("setContext", "canCut", canMove);
				vscode.commands.executeCommand(
					"setContext",
					"canCopy",
					canMove
				);
				vscode.commands.executeCommand(
					"setContext",
					"canPaste",
					canPaste
				);
				vscode.commands.executeCommand(
					"setContext",
					"canPasteInto",
					canPasteInto
				);
				vscode.commands.executeCommand(
					"setContext",
					"canInsert",
					canPasteInto
				);
			})
		);
	}

	dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
	}
}
