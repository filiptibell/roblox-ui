import * as vscode from "vscode";

import { Providers } from ".";

export class SelectionProvider implements vscode.Disposable {
	private readonly disposables: vscode.Disposable[];

	constructor(public readonly providers: Providers) {
		this.disposables = [
			vscode.window.onDidChangeVisibleTextEditors(() => this.revealVisibleEditors()),
			vscode.window.onDidChangeActiveTextEditor(() => this.revealActiveEditor()),
		];
	}

	dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
	}

	public async revealVisibleEditors() {
		for (const editor of vscode.window.visibleTextEditors) {
			const fsPath = editor.document.uri.fsPath;
			await this.providers.explorerTree.revealByPath(fsPath);
		}
	}

	public async revealActiveEditor() {
		const activeEditor = vscode.window.activeTextEditor;
		if (activeEditor) {
			const fsPath = activeEditor.document.uri.fsPath;
			await this.providers.explorerTree.revealByPath(fsPath, true);
		}
	}
}
