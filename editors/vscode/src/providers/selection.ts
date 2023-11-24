import * as vscode from "vscode";

import { ExplorerTreeProvider } from "../explorer";

export class SelectionProvider implements vscode.Disposable {
	private disposables: Array<vscode.Disposable> = new Array();

	constructor(private explorer: ExplorerTreeProvider) {
		// Reveal tree items when they become visible / active in the editor
		this.disposables.push(
			vscode.window.onDidChangeVisibleTextEditors(() => {
				for (const editor of vscode.window.visibleTextEditors) {
					const fsPath = editor.document.uri.fsPath;
					this.explorer.expandRevealPath(fsPath);
				}
			})
		);
		this.disposables.push(
			vscode.window.onDidChangeActiveTextEditor((editor) => {
				if (editor) {
					const fsPath = editor.document.uri.fsPath;
					const item = this.explorer.expandRevealPath(fsPath);
					if (item) {
						vscode.commands.executeCommand(
							"roblox-ui.explorer.select",
							item.workspacePath,
							item.domInstance.id
						);
					}
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
