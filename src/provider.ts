import * as vscode from "vscode";

import { getClassIconPath } from "./utils/icons";
import {
	SourcemapNode,
	findFilePath,
	getSourcemapNodeTreeOrder,
} from "./utils/sourcemap";

export class RojoTreeProvider implements vscode.TreeDataProvider<RojoTreeItem> {
	private rootSourcemaps: Map<string, SourcemapNode> = new Map();
	private rootTreeItems: Map<string, RojoTreeItem> = new Map();
	private rootFilePaths: Map<string, Map<string, RojoTreeItem>> = new Map();

	private _onDidChangeTreeData: vscode.EventEmitter<void> =
		new vscode.EventEmitter();
	readonly onDidChangeTreeData: vscode.Event<void> =
		this._onDidChangeTreeData.event;

	/**
	 * Update a workspace path with a new sourcemap.
	 *
	 * This will create a new sub-tree, or update an existing tree, if found.
	 */
	public update(workspacePath: string, rootNode: SourcemapNode) {
		const workspaceItem = new RojoTreeItem(workspacePath, rootNode, null);
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
		this.rootSourcemaps.delete(workspacePath);
		this.rootTreeItems.delete(workspacePath);
		this.rootFilePaths.delete(workspacePath);
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

	getParent(item: RojoTreeItem): RojoTreeItem | null {
		return item.getParent();
	}

	getChildren(item?: RojoTreeItem): RojoTreeItem[] {
		if (!item) {
			return [...this.rootTreeItems.keys()]
				.sort()
				.map((workspacePath) => this.rootTreeItems.get(workspacePath)!);
		} else {
			return item.getChildren();
		}
	}
}

export class RojoTreeItem extends vscode.TreeItem {
	private filePath: string | null;
	private fileIsScript: boolean;

	private parent: RojoTreeItem | null;
	private children: RojoTreeItem[] = [];

	constructor(
		workspaceRoot: string,
		node: SourcemapNode,
		parent: RojoTreeItem | null
	) {
		super(
			node.name,
			node.children
				? vscode.TreeItemCollapsibleState.Collapsed
				: vscode.TreeItemCollapsibleState.None
		);

		this.tooltip = node.className;
		this.iconPath = getClassIconPath(node.className);

		const [filePath, fileIsScript] = findFilePath(workspaceRoot, node);
		this.filePath = filePath;
		this.fileIsScript = fileIsScript || false;

		if (filePath && fileIsScript) {
			this.command = {
				title: "Open file",
				command: "vscode.open",
				arguments: [vscode.Uri.file(filePath)],
			};
		} else if (filePath) {
			this.contextValue = "projectRoot";
		}

		this.parent = parent;
		if (node.children) {
			this.children = [...node.children]
				.filter((child) => {
					return (
						child.className !== "Folder" || child.name !== "_Index"
					);
				})
				.sort((left, right) => {
					const leftOrder = getSourcemapNodeTreeOrder(left);
					const rightOrder = getSourcemapNodeTreeOrder(right);
					if (leftOrder !== rightOrder) {
						return leftOrder - rightOrder;
					} else {
						return left.name.localeCompare(right.name);
					}
				})
				.map((child) => new RojoTreeItem(workspaceRoot, child, this));
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
