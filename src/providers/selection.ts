import * as vscode from "vscode";

import { RojoTreeProvider } from "./tree";

export class SelectionProvider implements vscode.Disposable {
	private disposables: Array<vscode.Disposable> = new Array();

	constructor(
		treeView: vscode.TreeView<vscode.TreeItem>,
		treeDataProvider: RojoTreeProvider
	) {
		// Listen for focus changing to sync selection with our tree view items
		this.disposables.push(
			vscode.window.onDidChangeActiveTextEditor((e) => {
				if (e && treeView.visible) {
					const filePath = e.document.uri.fsPath;
					const fileItem = treeDataProvider.find(filePath);
					if (fileItem) {
						treeView.reveal(fileItem);
					}
				}
			})
		);

		// Reveal & select instances in the explorer when editors for them become visible
		let visibleEditors: Map<string, vscode.TextEditor> = new Map();
		this.disposables.push(
			vscode.window.onDidChangeVisibleTextEditors((editors) => {
				if (!treeView.visible) {
					return;
				}
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
	}

	dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
	}
}
