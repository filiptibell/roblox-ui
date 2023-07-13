import * as vscode from "vscode";

import { RojoTreeProvider } from "./explorer";

export class SelectionProvider implements vscode.Disposable {
	private disposables: Array<vscode.Disposable> = new Array();

	constructor(
		private treeView: vscode.TreeView<vscode.TreeItem>,
		private treeDataProvider: RojoTreeProvider
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
		treeView.onDidChangeVisibility((event) => {
			if (event.visible) {
				revealCurrentlyVisibleEditorsInTree();
			}
		});
		// Loading the tree view is asynchronous since it may call out to another
		// process or wait for fil I/O, so we also need to listen for it loading
		// since the user may have had a file open before the tree had loaded
		this.disposables.push(
			treeDataProvider.onInitialWorkspaceLoaded(async () => {
				// HACK: I don't know why this works, there seems to be
				// some race conditions here, but this will do for now
				let tries = 0;
				const retryRevealUntilSuccessOrTimeout = async () => {
					if (await revealCurrentlyVisibleEditorsInTree()) {
						setTimeout(revealCurrentlyVisibleEditorsInTree, 10);
						setTimeout(revealCurrentlyVisibleEditorsInTree, 25);
						return;
					}
					tries += 1;
					if (tries < 20) {
						setTimeout(retryRevealUntilSuccessOrTimeout, 50);
					}
				};
				retryRevealUntilSuccessOrTimeout();
			})
		);
		// If we only load one workspace, it is desirable to automatically
		// expand its root, since it is usually a single DataModel / game
		this.disposables.push(
			treeDataProvider.onAutoExpandRootDesired((root) => {
				this.expand(root);
			})
		);
	}

	async expand(item: vscode.TreeItem) {
		await this.treeView.reveal(item, {
			expand: true,
			focus: false,
			select: false,
		});
	}

	async reveal(item: vscode.TreeItem, force: true | void): Promise<boolean> {
		if (this.treeView.visible || force === true) {
			await this.treeView.reveal(item);
			return true;
		}
		return false;
	}

	async revealEditor(editor: vscode.TextEditor): Promise<boolean> {
		let path = editor.document.uri.fsPath;
		if (path.endsWith(".lua") || path.endsWith(".luau")) {
			const treeItem = await this.treeDataProvider.findTreeItem(path);
			if (treeItem) {
				return await this.reveal(treeItem);
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
