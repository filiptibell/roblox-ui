import * as vscode from "vscode";
import * as path from "path";

import { getClassIconPath } from "../../utils/icons";
import {
	SourcemapNode,
	findPrimaryFilePath,
	getSourcemapNodeTreeOrder,
} from "../../utils/sourcemap";

import { RojoTreeRoot } from "./root";

export class RojoTreeItem extends vscode.TreeItem {
	private filePath: string | null;
	private folderPath: string | null;

	private node: SourcemapNode;
	private parent: RojoTreeItem | null = null;
	private children: RojoTreeItem[] = [];

	private iconPathReal: string;
	private isLoading: boolean = false;

	constructor(
		root: RojoTreeRoot,
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
		this.node = node;
		this.tooltip = node.className;

		this.iconPathReal = getClassIconPath(root.apiDump, this.node.className);
		this.iconPath = isLoading
			? new vscode.ThemeIcon("loading~spin")
			: this.iconPathReal;

		const folderPath = node.folderPath
			? path.join(root.workspacePath, node.folderPath)
			: null;
		const filePath = findPrimaryFilePath(root.workspacePath, node);
		const fileIsScript = filePath
			? !filePath.endsWith(".project.json")
			: false;
		this.filePath = filePath;
		this.folderPath = folderPath;
		this.resourceUri = filePath
			? vscode.Uri.file(filePath)
			: folderPath
			? vscode.Uri.file(folderPath)
			: undefined;
		this.id = this.resourceUri ? this.resourceUri.fsPath : undefined;

		// Set description based on settings
		const fsPathFull = filePath
			? filePath
			: folderPath
			? folderPath
			: undefined;
		const fsPath = fsPathFull
			? fsPathFull.slice(root.workspacePath.length + 1)
			: undefined;
		const updateDescription = () => {
			const showClassNames = root.settingsProvider.get("showClassNames");
			const showFilePaths = root.settingsProvider.get("showFilePaths");
			if (showClassNames && showFilePaths) {
				if (fsPath) {
					this.description = `${node.className} - ${fsPath}`;
				} else {
					this.description = `${node.className}`;
				}
			} else if (showClassNames) {
				this.description = node.className;
			} else if (showFilePaths && fsPath) {
				this.description = fsPath;
			} else {
				this.description = undefined;
			}
		};
		updateDescription();

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
		if (parent && (filePath !== null || folderPath !== null)) {
			const info = root.apiDump.Classes.get(node.className);
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
		if (parent && parent.folderPath !== null) {
			contextPartials.add("canPasteSibling");
		}
		if (folderPath !== null) {
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
				.sort((left, right) => {
					const leftOrder = getSourcemapNodeTreeOrder(
						left,
						root.reflectionMetadata
					);
					const rightOrder = getSourcemapNodeTreeOrder(
						right,
						root.reflectionMetadata
					);
					if (leftOrder !== rightOrder) {
						return leftOrder - rightOrder;
					} else {
						return left.name.localeCompare(right.name);
					}
				})
				.map((child) => new RojoTreeItem(root, child, this));
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
