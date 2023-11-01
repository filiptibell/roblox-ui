import * as vscode from "vscode";

import { SourcemapNode } from "../../utils/sourcemap";

import { SettingsProvider } from "../settings";
import { IconsProvider } from "../icons";

import { RojoTreeRoot } from "./root";
import { RojoTreeItem } from "./item";
import { MetadataProvider } from "../metadata";

export class RojoTreeProvider
	implements vscode.TreeDataProvider<vscode.TreeItem>
{
	private roots: Map<string, RojoTreeRoot> = new Map();
	private loaded: Map<string, boolean> = new Map();
	private disposables: Array<vscode.Disposable> = new Array();
	private showDataModel: boolean = false;

	readonly _onDidChangeTreeData: vscode.EventEmitter<void | vscode.TreeItem> =
		new vscode.EventEmitter();
	public readonly onDidChangeTreeData: vscode.Event<void | vscode.TreeItem> =
		this._onDidChangeTreeData.event;

	readonly _onInitialWorkspaceLoaded: vscode.EventEmitter<string> =
		new vscode.EventEmitter();
	public readonly onInitialWorkspaceLoaded: vscode.Event<string> =
		this._onInitialWorkspaceLoaded.event;

	readonly _onAutoExpandRootDesired: vscode.EventEmitter<RojoTreeRoot> =
		new vscode.EventEmitter();
	public readonly onAutoExpandRootDesired: vscode.Event<RojoTreeRoot> =
		this._onAutoExpandRootDesired.event;

	constructor(
		public readonly settingsProvider: SettingsProvider,
		public readonly metadataProvider: MetadataProvider,
		public readonly iconsProvider: IconsProvider
	) {
		const show = !!settingsProvider.get("explorer.showDataModel");
		this.showDataModel = show;
		this.disposables.push(
			settingsProvider.listen("explorer.showDataModel", () => {
				const show = !!settingsProvider.get("explorer.showDataModel");
				if (this.showDataModel !== show) {
					this.showDataModel = show;
					this._onDidChangeTreeData.fire();
				}
			})
		);
	}

	private findRoot(workspacePath: string) {
		return this.roots.get(workspacePath);
	}

	private createRoot(workspacePath: string) {
		const root = new RojoTreeRoot(
			workspacePath,
			this.settingsProvider,
			this.metadataProvider,
			this.iconsProvider,
			this,
			this._onDidChangeTreeData
		);
		this.roots.set(workspacePath, root);
		this.loaded.set(workspacePath, false);
		return root;
	}

	private updateInitialWorkspaceLoaded(workspacePath: string) {
		if (this.loaded.get(workspacePath)) {
			return;
		}
		const root = this.findRoot(workspacePath);
		if (root && root.treeHasLoaded()) {
			this.loaded.set(workspacePath, true);
			this._onInitialWorkspaceLoaded.fire(workspacePath);
			this._onDidChangeTreeData.fire();
		}
	}

	private deleteRoot(workspacePath: string) {
		const root = this.findRoot(workspacePath);
		if (root) {
			root.dispose();
			this.roots.delete(workspacePath);
			this.loaded.delete(workspacePath);
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
	public setLoading(workspacePath: string, projectPath: string | undefined) {
		let root = this.findRoot(workspacePath);
		if (root) {
			root.setLoading(projectPath);
		} else {
			root = this.createRoot(workspacePath);
			root.setLoading(projectPath);
			this._onDidChangeTreeData.fire();
		}
	}

	/**
	 * Mark a workspace path as no longer loading.
	 */
	public clearLoading(workspacePath: string) {
		const root = this.findRoot(workspacePath);
		if (root) {
			root.clearLoading().then(() => {
				this.updateInitialWorkspaceLoaded(workspacePath);
			});
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
	 * This will also automatically clear any loading states, but not error states.
	 */
	public update(
		workspacePath: string,
		rootNode: SourcemapNode,
		forced: boolean | void
	) {
		// HACK: If this is the only workspace root we have, we can
		// automatically expand it so that the user does not have to do
		// it themselves, improves the experience for like 99% of users
		const expandSingleRoot = () => {
			let root = this.findRoot(workspacePath);
			if (root !== undefined && this.roots.size === 1) {
				root.getChildren().then(() => {
					if (root !== undefined) {
						this._onAutoExpandRootDesired.fire(root);
					}
				});
			}
		};
		let root = this.findRoot(workspacePath);
		if (root) {
			root.updateTree(rootNode, forced).then(() => {
				expandSingleRoot();
				this.updateInitialWorkspaceLoaded(workspacePath);
			});
		} else {
			root = this.createRoot(workspacePath);
			root.updateTree(rootNode, forced).then(() => {
				this._onDidChangeTreeData.fire();
				expandSingleRoot();
				this.updateInitialWorkspaceLoaded(workspacePath);
			});
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
	 * Find a tree item for the given filesystem path.
	 */
	public async findTreeItem(
		filePath: string,
		pathIsRelative: boolean | undefined | null | void
	): Promise<vscode.TreeItem | null> {
		const promises: Array<Promise<vscode.TreeItem | null>> = new Array();
		for (const root of this.roots.values()) {
			promises.push(root.findTreeItem(filePath, pathIsRelative));
		}
		const results = await Promise.all(promises);
		for (const result of results) {
			if (result !== null) {
				return result;
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

	async getChildren(item?: RojoTreeItem): Promise<vscode.TreeItem[]> {
		if (!item) {
			if (!this.showDataModel && this.roots.size <= 1) {
				let firstPath = [...this.roots.keys()][0];
				let firstRoot = this.findRoot(firstPath);
				if (firstRoot && firstRoot.isDataModel()) {
					return await firstRoot.getChildren();
				}
			}
			return [...this.roots.keys()]
				.sort()
				.map((path) => this.findRoot(path)!);
		} else {
			return await item.getChildren();
		}
	}

	dispose() {
		const workspacePaths = [...this.roots.keys()];
		for (const workspacePath of workspacePaths.values()) {
			this.deleteRoot(workspacePath);
		}
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
	}
}
