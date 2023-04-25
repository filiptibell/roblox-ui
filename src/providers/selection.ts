import * as vscode from "vscode";

import { RojoTreeProvider } from "./tree";

export class SelectionProvider implements vscode.Disposable {
	private disposables: Array<vscode.Disposable> = new Array();

	constructor(
		treeView: vscode.TreeView<vscode.TreeItem>,
		treeDataProvider: RojoTreeProvider
	) {
		// Reveal instances in the explorer when editors for them become visible
		let visibleEditors: Map<string, vscode.TextEditor> = new Map();
		this.disposables.push(
			vscode.window.onDidChangeVisibleTextEditors((editors) => {
				const newEditors = new Map();
				for (const editor of editors) {
					const editorPath = editor.document.uri.fsPath;
					newEditors.set(editorPath, editor);
					if (!visibleEditors.has(editorPath)) {
						const treeItem = treeDataProvider.find(editorPath);
						if (treeItem) {
							treeView.reveal(treeItem, {
								select: true,
								focus: true,
							});
						}
					}
				}
				visibleEditors = newEditors;
			})
		);

		// Update actions context (copy, paste, insert) when the explorer selection changes
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
