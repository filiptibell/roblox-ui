import * as vscode from "vscode";

import { RojoTreeProvider } from "./provider";

let selectionDisposable: vscode.Disposable | null;

export const connectSelection = (
	treeView: vscode.TreeView<vscode.TreeItem>,
	treeDataProvider: RojoTreeProvider
) => {
	selectionDisposable = treeView.onDidChangeSelection((event) => {
		const selected: Array<any> = Array.from(event.selection);

		let canMove = false;
		let canPaste = false;
		let canPasteInto = false;
		try {
			canMove = selected.every((item) => item.canMove());
			canPaste = selected.every((item) => item.canPaste());
			canPasteInto = selected.every((item) => item.canPasteInto());
		} catch {}

		vscode.commands.executeCommand("setContext", "canCut", canMove);
		vscode.commands.executeCommand("setContext", "canCopy", canMove);
		vscode.commands.executeCommand("setContext", "canPaste", canPaste);
		vscode.commands.executeCommand(
			"setContext",
			"canPasteInto",
			canPasteInto
		);
		vscode.commands.executeCommand("setContext", "canInsert", canPasteInto);
	});
};

export const disconnectSelection = () => {
	if (selectionDisposable) {
		selectionDisposable.dispose();
		selectionDisposable = null;
	}
};
