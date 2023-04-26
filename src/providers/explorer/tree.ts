import * as vscode from "vscode";

import { SourcemapNode } from "../../utils/sourcemap";
import { RobloxApiDump, RobloxReflectionMetadata } from "../../web/roblox";
import { SettingsProvider } from "../settings";

import { RojoTreeRoot } from "./root";
import { RojoTreeItem } from "./item";

export class RojoTreeProvider
	implements vscode.TreeDataProvider<vscode.TreeItem>
{
	private roots: Map<string, RojoTreeRoot> = new Map();

	readonly _onDidChangeTreeData: vscode.EventEmitter<void | vscode.TreeItem> =
		new vscode.EventEmitter();
	readonly onDidChangeTreeData: vscode.Event<void | vscode.TreeItem> =
		this._onDidChangeTreeData.event;

	constructor(
		public readonly settingsProvider: SettingsProvider,
		public readonly apiDump: RobloxApiDump,
		public readonly reflectionMetadata: RobloxReflectionMetadata
	) {}

	private findRoot(workspacePath: string) {
		return this.roots.get(workspacePath);
	}

	private createRoot(workspacePath: string) {
		const root = new RojoTreeRoot(
			workspacePath,
			this.settingsProvider,
			this,
			this.apiDump,
			this.reflectionMetadata,
			this._onDidChangeTreeData
		);
		this.roots.set(workspacePath, root);
		return root;
	}

	private deleteRoot(workspacePath: string) {
		const root = this.findRoot(workspacePath);
		if (root) {
			root.dispose();
			this.roots.delete(workspacePath);
			return true;
		} else {
			return false;
		}
	}

	/**
	 * Mark a workspace path as currently loading.
	 *
	 * This will display a loading spinner in the tree view.
	 */
	public setLoading(workspacePath: string) {
		let root = this.findRoot(workspacePath);
		if (root) {
			root.setLoading();
		} else {
			root = this.createRoot(workspacePath);
			root.setLoading();
			this._onDidChangeTreeData.fire();
		}
	}

	/**
	 * Mark a workspace path as no longer loading.
	 */
	public clearLoading(workspacePath: string) {
		const root = this.findRoot(workspacePath);
		if (root) {
			root.clearLoading();
		}
	}

	/**
	 * Mark a workspace path as errored.
	 *
	 * This will display an error icon in the tree view.
	 */
	public setError(workspacePath: string, errorMessage: string) {
		let root = this.findRoot(workspacePath);
		if (root) {
			root.setError(errorMessage);
		} else {
			root = this.createRoot(workspacePath);
			root.setError(errorMessage);
			this._onDidChangeTreeData.fire();
		}
	}

	/**
	 * Mark a workspace path as no longer errored.
	 */
	public clearError(workspacePath: string) {
		const root = this.findRoot(workspacePath);
		if (root) {
			root.clearError();
		}
	}

	/**
	 * Update a workspace path with a new sourcemap.
	 *
	 * This will create a new sub-tree, or update an existing tree, if found.
	 *
	 * This will also automatically clear any loading or errored states.
	 */
	public update(workspacePath: string, rootNode: SourcemapNode) {
		let root = this.findRoot(workspacePath);
		if (root) {
			root.update(rootNode);
		} else {
			root = this.createRoot(workspacePath);
			root.update(rootNode);
			this._onDidChangeTreeData.fire();
		}
	}

	/**
	 * Delete a workspace path.
	 *
	 * This will remove all tree items and references to the workspace.
	 */
	public delete(workspacePath: string) {
		if (this.deleteRoot(workspacePath)) {
			this._onDidChangeTreeData.fire();
		}
	}

	/**
	 * Find a workspace tree item from the given file path.
	 *
	 * This will search all currently known workspace paths.
	 */
	public find(filePath: string): RojoTreeItem | null {
		for (const root of this.roots.values()) {
			const item = root.find(filePath);
			if (item) {
				return item;
			}
		}
		return null;
	}

	getTreeItem(item: RojoTreeItem): vscode.TreeItem {
		return item;
	}

	getParent(item: RojoTreeItem): vscode.TreeItem | null {
		return item.getParent();
	}

	getChildren(item?: RojoTreeItem): vscode.TreeItem[] {
		if (!item) {
			return [...this.roots.keys()]
				.sort()
				.map((path) => this.findRoot(path)!);
		} else {
			return item.getChildren();
		}
	}

	dispose() {
		const workspacePaths = [...this.roots.keys()];
		for (const workspacePath of workspacePaths.values()) {
			this.deleteRoot(workspacePath);
		}
	}
}
