import * as vscode from "vscode";

import { ExplorerTreeProvider } from "../explorer";

export class SelectionProvider implements vscode.Disposable {
	private readonly disposables: Array<vscode.Disposable> = new Array();

	constructor(private explorer: ExplorerTreeProvider) {
		this.disposables.push(
			vscode.window.onDidChangeVisibleTextEditors(
				this.revealVisibleEditors
			)
		);
		this.disposables.push(
			vscode.window.onDidChangeActiveTextEditor(this.revealActiveEditor)
		);
		this.revealVisibleEditors();
		this.revealActiveEditor();
	}

	dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
	}

	public async revealVisibleEditors() {
		for (const editor of vscode.window.visibleTextEditors) {
			const fsPath = editor.document.uri.fsPath;
			await this.explorer.revealByPath(fsPath);
		}
	}

	public async revealActiveEditor() {
		const activeEditor = vscode.window.activeTextEditor;
		if (activeEditor) {
			const fsPath = activeEditor.document.uri.fsPath;
			await this.explorer.revealByPath(fsPath, true);
		}
	}
}
