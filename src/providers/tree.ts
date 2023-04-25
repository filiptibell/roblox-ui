import * as vscode from "vscode";
import * as path from "path";

import {
	GitExtension,
	RepositoryState as GitRepositoryState,
} from "../types/git";

import { getClassIconPath } from "../utils/icons";
import {
	SourcemapNode,
	findPrimaryFilePath,
	getSourcemapNodeTreeOrder,
} from "../utils/sourcemap";

import { RobloxApiDump } from "../web/robloxApiDump";
import { RobloxReflectionMetadata } from "../web/robloxReflectionMetadata";
import { SettingsProvider } from "./settings";

export class RojoTreeProvider
	implements vscode.TreeDataProvider<vscode.TreeItem>
{
	private rootLoadStates: Map<string, boolean> = new Map();
	private rootSourcemaps: Map<string, SourcemapNode> = new Map();
	private rootTreeItems: Map<string, RojoTreeItem | LoadingTreeItem> =
		new Map();
	private rootFilePaths: Map<string, Map<string, RojoTreeItem>> = new Map();

	private _onDidChangeTreeData: vscode.EventEmitter<void | vscode.TreeItem> =
		new vscode.EventEmitter();
	readonly onDidChangeTreeData: vscode.Event<void | vscode.TreeItem> =
		this._onDidChangeTreeData.event;

	private gitRepos: Map<string, GitRepositoryState | null> = new Map();
	private gitDisposables: Map<string, vscode.Disposable> = new Map();

	constructor(
		public readonly settingsProvider: SettingsProvider,
		public readonly apiDump: RobloxApiDump,
		public readonly reflectionMetadata: RobloxReflectionMetadata
	) {}

	tryInitGitRepo(workspacePath: string) {
		try {
			const gitExtension =
				vscode.extensions.getExtension<GitExtension>("vscode.git");
			if (gitExtension && gitExtension.exports.enabled) {
				const gitApi = gitExtension.exports.getAPI(1);
				if (gitApi && !this.gitRepos.has(workspacePath)) {
					const repo = gitApi.getRepository(
						vscode.Uri.file(workspacePath)
					);
					if (repo) {
						this.gitRepos.set(workspacePath, repo.state);
						this.gitDisposables.set(
							workspacePath,
							repo.state.onDidChange(() => {
								this.gitRepos.set(workspacePath, repo.state);
								// FUTURE: Git decorations on stuff? git actions?
							})
						);
					} else {
						this.gitRepos.set(workspacePath, null);
					}
				}
			}
		} catch {}
	}

	/**
	 * Mark a workspace path as currently loading.
	 *
	 * This will display a loading spinner in the tree view.
	 */
	public setLoading(workspacePath: string) {
		this.tryInitGitRepo(workspacePath);
		this.rootLoadStates.set(workspacePath, true);
		const existingItem = this.rootTreeItems.get(workspacePath);
		if (existingItem) {
			if (existingItem.setIsLoading(true)) {
				this._onDidChangeTreeData.fire(existingItem);
			}
		} else {
			const workspaceItem = new LoadingTreeItem(workspacePath);
			this.rootTreeItems.set(workspacePath, workspaceItem);
			this._onDidChangeTreeData.fire();
		}
	}

	/**
	 * Update a workspace path with a new sourcemap.
	 *
	 * This will create a new sub-tree, or update an existing tree, if found.
	 */
	public update(workspacePath: string, rootNode: SourcemapNode) {
		this.tryInitGitRepo(workspacePath);
		const workspaceItem = new RojoTreeItem(
			this.settingsProvider,
			this,
			workspacePath,
			rootNode
		);
		this.rootLoadStates.delete(workspacePath);
		this.rootSourcemaps.set(workspacePath, rootNode);
		this.rootTreeItems.set(workspacePath, workspaceItem);
		this.rootFilePaths.set(workspacePath, workspaceItem.gatherFilePaths());
		this._onDidChangeTreeData.fire();
	}

	/**
	 * Delete a workspace path.
	 *
	 * This will remove all tree items and references to the workspace.
	 */
	public delete(workspacePath: string) {
		const disposable = this.gitDisposables.get(workspacePath);
		if (disposable) {
			disposable.dispose();
		}
		this.rootLoadStates.delete(workspacePath);
		this.rootSourcemaps.delete(workspacePath);
		this.rootTreeItems.delete(workspacePath);
		this.rootFilePaths.delete(workspacePath);
		this.gitRepos.delete(workspacePath);
		this.gitDisposables.delete(workspacePath);
		this._onDidChangeTreeData.fire();
	}

	/**
	 * Find a workspace tree item from the given file path.
	 *
	 * This will search all currently known workspace paths.
	 */
	public find(filePath: string): RojoTreeItem | null {
		for (const map of this.rootFilePaths.values()) {
			const item = map.get(filePath);
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
			const keys = new Set([...this.rootTreeItems.keys()]);
			for (const key of this.rootLoadStates.keys()) {
				keys.add(key);
			}
			return Array.from(keys)
				.sort()
				.map((workspacePath) => this.rootTreeItems.get(workspacePath)!);
		} else {
			return item.getChildren();
		}
	}

	dispose() {
		const disposables = this.gitDisposables.entries();
		for (const [key, disposable] of disposables) {
			disposable.dispose();
			this.gitDisposables.delete(key);
		}
	}
}

export class RojoTreeItem extends vscode.TreeItem {
	private filePath: string | null;
	private folderPath: string | null;
	private fileIsScript: boolean;

	private workspaceRoot: string;
	private node: SourcemapNode;
	private parent: RojoTreeItem | null = null;
	private children: RojoTreeItem[] = [];

	private iconPathReal: string;
	private isLoading: boolean = false;

	constructor(
		settingsProvider: SettingsProvider,
		treeProvider: RojoTreeProvider,
		workspacePath: string,
		node: SourcemapNode,
		parent: RojoTreeItem | undefined | null | void,
		isLoading: boolean | undefined | null | void
	) {
		super(
			node.name,
			node.children
				? vscode.TreeItemCollapsibleState.Collapsed
				: vscode.TreeItemCollapsibleState.None
		);

		this.setIsLoading(isLoading);
		this.workspaceRoot = workspacePath;
		this.node = node;
		this.tooltip = node.className;

		this.iconPathReal = getClassIconPath(
			treeProvider.apiDump,
			this.node.className
		);
		this.iconPath = this.isLoading
			? new vscode.ThemeIcon("loading~spin")
			: this.iconPathReal;

		const folderPath = node.folderPath
			? path.join(workspacePath, node.folderPath)
			: null;
		const filePath = findPrimaryFilePath(workspacePath, node);
		const fileIsScript = filePath
			? !filePath.endsWith(".project.json")
			: false;
		this.filePath = filePath;
		this.folderPath = folderPath;
		this.fileIsScript = fileIsScript || false;
		this.resourceUri = filePath
			? vscode.Uri.file(filePath)
			: folderPath
			? vscode.Uri.file(folderPath)
			: undefined;

		if (settingsProvider.get("showFilePaths")) {
			const fsPath = filePath
				? filePath
				: folderPath
				? folderPath
				: undefined;
			if (fsPath) {
				this.description = fsPath.slice(workspacePath.length + 1);
			}
		}

		// Set context value for menu actions such as copy,
		// paste, insert object, rename, ... to appear correctly
		const contextPartials = new Set();
		if (filePath) {
			if (fileIsScript) {
				this.command = {
					title: "Open file",
					command: "vscode.open",
					arguments: [vscode.Uri.file(filePath)],
				};
				contextPartials.add("instance");
			} else {
				contextPartials.add("projectFile");
			}
		} else if (folderPath) {
			contextPartials.add("instance");
		}
		if (this.filePath !== null || this.folderPath !== null) {
			const info = treeProvider.apiDump.Classes.get(node.className);
			vscode.window.showInformationMessage(
				`${JSON.stringify(info?.Tags)}`
			);
			if (
				!info ||
				!(
					info.Name === "DataModel" ||
					info.Tags.find((tag) => tag === "Service")
				)
			) {
				contextPartials.add("canMove");
			}
		}
		if (
			this.parent !== null &&
			this.parent.folderPath !== null &&
			!node.folderPath
		) {
			contextPartials.add("canPaste");
		}
		if (this.folderPath !== null) {
			contextPartials.add("canPasteInto");
			if (contextPartials.has("instance")) {
				contextPartials.add("canInsertObject");
			} else if (contextPartials.has("projectFile")) {
				// DataModel nodes that have a folderPath are safe
				// to add services into, the folder path is a confirmed
				// shared prefix folder where all current services exist
				if (node.className === "DataModel" && node.folderPath) {
					contextPartials.add("canInsertService");
				}
			}
		}
		this.contextValue = Array.from(contextPartials.values()).join(";");

		// Set parent reference and create child tree items
		if (parent) {
			this.parent = parent;
		}
		if (node.children) {
			this.children = [...node.children]
				.filter((child) => {
					return (
						child.className !== "Folder" || child.name !== "_Index"
					);
				})
				.sort((left, right) => {
					const leftOrder = getSourcemapNodeTreeOrder(
						left,
						treeProvider.reflectionMetadata
					);
					const rightOrder = getSourcemapNodeTreeOrder(
						right,
						treeProvider.reflectionMetadata
					);
					if (leftOrder !== rightOrder) {
						return leftOrder - rightOrder;
					} else {
						return left.name.localeCompare(right.name);
					}
				})
				.map(
					(child) =>
						new RojoTreeItem(
							settingsProvider,
							treeProvider,
							workspacePath,
							child,
							this
						)
				);
		}
	}

	/**
	 * Set if this tree item is currently loading or not.
	 *
	 * When the tree item is loading, it will show a spinning loading indicator.
	 */
	public setIsLoading(
		isLoadingArg: boolean | undefined | null | void
	): boolean {
		const isLoading = isLoadingArg ? true : false;
		if (this.isLoading !== isLoading) {
			this.isLoading = isLoading;
			this.iconPath = this.isLoading
				? new vscode.ThemeIcon("loading~spin")
				: this.iconPathReal;
			return true;
		} else {
			return false;
		}
	}

	/**
	 * Gathers a map of file path -> tree items.
	 *
	 * This will include the tree item it is called on.
	 */
	public gatherFilePaths(
		map: Map<string, RojoTreeItem> | void
	): Map<string, RojoTreeItem> {
		if (map === undefined) {
			map = new Map();
		}
		if (this.filePath) {
			map.set(this.filePath, this);
		}
		if (this.children) {
			for (const child of this.children.values()) {
				child.gatherFilePaths(map);
			}
		}
		return map;
	}

	/**
	 * Opens the file path associated with this tree item.
	 *
	 * @returns `true` if the file was opened, `false` otherwise.
	 */
	public openFile(): boolean {
		if (this.filePath) {
			vscode.commands.executeCommand(
				"vscode.open",
				vscode.Uri.file(this.filePath)
			);
			return true;
		} else {
			return false;
		}
	}

	/**
	 * Gets the file path associated with this tree item.
	 */
	public getFilePath(): string | null {
		return this.filePath;
	}

	/**
	 * Gets the folder path associated with this tree item.
	 */
	public getFolderPath(): string | null {
		return this.folderPath;
	}

	/**
	 * Gets the parent tree item for this tree item, if any.
	 */
	public getParent(): RojoTreeItem | null {
		return this.parent;
	}

	/**
	 * Gets a list of all child tree items for this tree item.
	 *
	 * This list of children is unique and can be mutated freely.
	 */
	public getChildren(): RojoTreeItem[] {
		return [...this.children];
	}
}

export class LoadingTreeItem extends vscode.TreeItem {
	constructor(workspacePath: string) {
		let label;
		try {
			const wpath = path.parse(workspacePath);
			label = `Loading workspace - ${wpath.name}`;
		} catch {
			label = "Loading workspace";
		}
		super(label);
		this.iconPath = new vscode.ThemeIcon("loading~spin");
	}

	public setIsLoading(_isLoading: boolean | undefined | null | void) {}
}
