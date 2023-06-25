import * as vscode from "vscode";

import { RojoTreeProvider } from "./explorer";

export class SelectionProvider implements vscode.Disposable {
	private disposables: Array<vscode.Disposable> = new Array();

	constructor(
		treeView: vscode.TreeView<vscode.TreeItem>,
		treeDataProvider: RojoTreeProvider
	) {
		this.disposables.push(
			vscode.window.onDidChangeActiveTextEditor(async (editor) => {
				if (!editor || !treeView.visible) {
					return;
				}
				let path = editor.document.uri.fsPath;
				if (!path.endsWith(".lua") && !path.endsWith(".luau")) {
					return;
				}
				const treeItem = await treeDataProvider.findTreeItem(path);
				if (treeItem) {
					treeView.reveal(treeItem);
				}
			})
		);
	}

	dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
	}
}
