import * as vscode from "vscode";

import { RojoTreeProvider } from "./explorer";
import { ExplorerTreeProvider } from "../explorer";

export class SelectionProvider implements vscode.Disposable {
	private disposables: Array<vscode.Disposable> = new Array();
	private treeViewVisible: boolean = false;

	constructor(
		private treeView: vscode.TreeView<vscode.TreeItem>,
		private treeDataProvider: ExplorerTreeProvider
	) {
		const revealCurrentlyVisibleEditorsInTree = async () => {
			const promises: Promise<boolean>[] = [];
			vscode.window.visibleTextEditors.forEach((editor) =>
				promises.push(this.revealEditor(editor))
			);
			const results = await Promise.all(promises);
			return results.find((val) => val === true);
		};
		revealCurrentlyVisibleEditorsInTree();

		// Reveal tree items when they become visible / active in the editor
		this.disposables.push(
			vscode.window.onDidChangeVisibleTextEditors(() =>
				revealCurrentlyVisibleEditorsInTree()
			)
		);
		this.disposables.push(
			vscode.window.onDidChangeActiveTextEditor((editor) => {
				if (editor) {
					this.revealEditor(editor);
				}
			})
		);

		// If our tree view was not previously visible we would
		// not have revealed any tree items currently being edited,
		// so we need to listen for the tree view becoming visible
		this.treeViewVisible = !!treeView.visible;
		treeView.onDidChangeVisibility(() => {
			const newVisible = !!treeView.visible;
			if (this.treeViewVisible !== newVisible) {
				this.treeViewVisible = newVisible;
				revealCurrentlyVisibleEditorsInTree();
			}
		});
	}

	async expand(item: vscode.TreeItem) {
		await this.treeView.reveal(item, {
			expand: true,
			focus: false,
			select: false,
		});
	}

	async reveal(item: vscode.TreeItem): Promise<boolean> {
		if (this.treeViewVisible) {
			await this.treeView.reveal(item);
			return true;
		}
		return false;
	}

	async revealEditor(editor: vscode.TextEditor): Promise<boolean> {
		let path = editor.document.uri.fsPath;
		for (const workspacePath of this.treeDataProvider.getWorkspacePaths()) {
			const domInstance = await this.treeDataProvider.findByPath(
				workspacePath,
				path
			);
			if (domInstance) {
				const treeItem = await this.treeDataProvider.findExplorerItem(
					workspacePath,
					domInstance.id
				);
				if (treeItem) {
					return await this.reveal(treeItem);
				}
			}
		}
		return false;
	}

	dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
	}
}
