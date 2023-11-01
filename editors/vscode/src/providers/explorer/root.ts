import * as vscode from "vscode";

import { SourcemapNode } from "../../utils/sourcemap";

import { SettingsProvider } from "../settings";
import { IconsProvider } from "../icons";

import { RojoTreeProvider } from "./tree";
import { RojoTreeItem } from "./item";
import {
	TreeItemPropChanges,
	getNullProps,
	getErroredProps,
	getLoadingProps,
} from "./props";
import { MetadataProvider } from "../metadata";

export class RojoTreeRoot extends vscode.TreeItem implements vscode.Disposable {
	private isLoading: boolean = true;

	private errorMessage: string | undefined;
	private projectPath: string | undefined;
	private treeItem: RojoTreeItem | undefined;

	private sourcemap: SourcemapNode | undefined;
	private sourcemapPathsMap: Map<string, Array<SourcemapNode>> | undefined;
	private sourcemapParentsMap: Map<SourcemapNode, SourcemapNode> | undefined;
	private sourcemapChangePending: boolean = false;

	dispose() {}

	constructor(
		public readonly workspacePath: string,
		public readonly settingsProvider: SettingsProvider,
		public readonly metadataProvider: MetadataProvider,
		public readonly iconsProvider: IconsProvider,
		public readonly treeProvider: RojoTreeProvider,
		private readonly eventEmitter: vscode.EventEmitter<void | vscode.TreeItem>
	) {
		super("<<<ROOT>>>");
		this.id = workspacePath;
		this.refreshTreeItem();
	}

	private async refreshTreeItem(forced: boolean | void) {
		let newProps: TreeItemPropChanges = {};
		let rootChanged = false;
		let childrenChanged = false;

		if (this.errorMessage) {
			newProps = getErroredProps(this.workspacePath, this.errorMessage);
		} else if (
			(this.sourcemap && this.sourcemapChangePending) ||
			this.treeItem
		) {
			if (this.sourcemapChangePending) {
				this.sourcemapChangePending = false;
				newProps = getNullProps();
				let treeItem = this.treeItem;
				try {
					if (!treeItem) {
						treeItem = new RojoTreeItem(this, this.eventEmitter);
					}
					childrenChanged = await treeItem.update(
						this.sourcemap,
						forced
					);
					this.treeItem = treeItem;
				} catch (err) {
					this.setError(`${err}`);
					await this.refreshTreeItem();
					return;
				}
				for (const key of Object.keys(newProps)) {
					const untyped = newProps as any;
					const value = (treeItem as any)[key];
					if (value !== undefined && value !== untyped[key]) {
						untyped[key] = value;
					}
				}
			}
			if (this.isLoading) {
				const loadingProps = getLoadingProps(
					this.workspacePath,
					this.projectPath
				);
				if (loadingProps.iconPath) {
					newProps.iconPath = loadingProps.iconPath;
				}
			}
		} else if (this.isLoading) {
			newProps = getLoadingProps(this.workspacePath, this.projectPath);
		}

		for (const [key, value] of Object.entries(newProps)) {
			const untyped = this as any;
			if (untyped[key] !== value) {
				untyped[key] = value;
				rootChanged = true;
			}
		}

		if (childrenChanged) {
			const smapChildren = this.sourcemap!.children;
			const newCollapsibleState =
				smapChildren && smapChildren.length > 0
					? this.collapsibleState ===
					  vscode.TreeItemCollapsibleState.Expanded
						? vscode.TreeItemCollapsibleState.Expanded
						: vscode.TreeItemCollapsibleState.Collapsed
					: vscode.TreeItemCollapsibleState.None;
			if (this.collapsibleState !== newCollapsibleState) {
				this.collapsibleState = newCollapsibleState;
				rootChanged = true;
			}
		}

		if (!this.resourceUri && this.projectPath) {
			this.resourceUri = vscode.Uri.file(this.projectPath);
			rootChanged = true;
		}
		if (this.contextValue) {
			if (this.projectPath && !this.contextValue.match("projectFile")) {
				this.contextValue += ";projectFile";
				rootChanged = true;
			}
		} else if (this.projectPath) {
			this.contextValue = "projectFile";
			rootChanged = true;
		}

		if (rootChanged || childrenChanged) {
			this.eventEmitter.fire(this);
		}
	}

	public isDataModel(): boolean {
		return this.sourcemap?.className === "DataModel";
	}

	public treeHasLoaded(): boolean {
		return !this.isLoading && this.treeItem !== undefined;
	}

	public async setLoading(projectPath: string | undefined): Promise<boolean> {
		if (!this.isLoading || this.projectPath !== projectPath) {
			this.isLoading = true;
			this.projectPath = projectPath;
			this.clearError();
			await this.refreshTreeItem();
			return true;
		}
		return false;
	}

	public async clearLoading(): Promise<boolean> {
		if (this.isLoading) {
			this.isLoading = false;
			this.projectPath = undefined;
			await this.refreshTreeItem();
			return true;
		}
		return false;
	}

	public async setError(errorMessage: string) {
		if (this.errorMessage !== errorMessage) {
			this.errorMessage = errorMessage;
			this.clearTree();
			this.clearLoading();
			await this.refreshTreeItem();
		}
	}

	public async clearError() {
		if (this.errorMessage) {
			this.errorMessage = undefined;
			await this.refreshTreeItem();
		}
	}

	public async updateTree(rootNode: SourcemapNode, forced: boolean | void) {
		this.isLoading = false;
		this.sourcemap = rootNode;
		this.sourcemapChangePending = true;
		if (this.sourcemapPathsMap) {
			this.sourcemapPathsMap.clear();
			this.sourcemapPathsMap = undefined;
		}
		if (this.sourcemapParentsMap) {
			this.sourcemapParentsMap.clear();
			this.sourcemapParentsMap = undefined;
		}
		await this.refreshTreeItem(forced);
	}

	public async clearTree() {
		if (this.treeItem) {
			this.treeItem = undefined;
			this.sourcemap = undefined;
			this.sourcemapChangePending = true;
			if (this.sourcemapPathsMap) {
				this.sourcemapPathsMap.clear();
				this.sourcemapPathsMap = undefined;
			}
			if (this.sourcemapParentsMap) {
				this.sourcemapParentsMap.clear();
				this.sourcemapParentsMap = undefined;
			}
			await this.refreshTreeItem();
		}
	}

	public async findTreeItem(
		filePath: string,
		pathIsRelative: boolean | undefined | null | void
	): Promise<vscode.TreeItem | null> {
		if (!this.sourcemap) {
			return null;
		}

		// Make sure we have a tree item, or try to create it by refreshing
		if (!this.treeItem) {
			await this.refreshTreeItem();
		}
		if (!this.treeItem) {
			return null;
		}

		// Make sure we have maps between sourcemap nodes,
		// their paths and parents, or try to create them
		if (!this.sourcemapPathsMap) {
			this.sourcemapPathsMap = new Map();
			createSourcemapPathsMap(this.sourcemapPathsMap, this.sourcemap);
		}
		if (!this.sourcemapParentsMap) {
			this.sourcemapParentsMap = new Map();
			createSourcemapParentsMap(
				this.sourcemapParentsMap,
				this.sourcemap,
				undefined
			);
		}

		const relPath = pathIsRelative
			? filePath
			: filePath.slice(this.workspacePath.length + 1);
		const nodes = this.sourcemapPathsMap.get(relPath);
		if (nodes) {
			for (const node of nodes) {
				// Create a path of sourcemap nodes from the root of the
				// sourcemap tree to the leaf node that we are looking for
				const nodePath: Array<SourcemapNode> = new Array();
				let nodePathCurrent: SourcemapNode | undefined = node;
				while (nodePathCurrent) {
					nodePath.unshift(nodePathCurrent);
					nodePathCurrent =
						this.sourcemapParentsMap.get(nodePathCurrent);
				}

				nodePath.shift(); // Remove root

				// Traverse the path in order, with the corrensponding tree item at the same time:
				// tree root -> tree item 1 -> tree item 2 -> ... -> tree leaf
				// smap root -> smap item 1 -> smap item 2 -> ... -> smap leaf
				let currentTreeItem = this.treeItem;
				let currentNodeInPath = nodePath.shift();
				while (currentNodeInPath !== undefined) {
					let found = undefined;
					const children = await currentTreeItem.getChildren();
					for (const child of children) {
						if (child.getNode() === currentNodeInPath) {
							found = child;
							break;
						}
					}
					if (found) {
						currentTreeItem = found;
						currentNodeInPath = nodePath.shift();
						// End of path and everything matched, means we must
						// have found the leaf tree item we were looking for
						if (currentNodeInPath === undefined) {
							return currentTreeItem;
						}
					} else {
						break;
					}
				}
			}
		}
		return null;
	}

	openFile(): boolean {
		if (this.projectPath) {
			vscode.commands.executeCommand(
				"vscode.open",
				vscode.Uri.file(this.projectPath)
			);
			return true;
		} else {
			return false;
		}
	}

	getFilePath(): string | null {
		return this.treeItem ? this.treeItem.getFilePath() : null;
	}

	getFolderPath(): string | null {
		return this.treeItem ? this.treeItem.getFolderPath() : null;
	}

	getTreeItem() {
		return undefined;
	}

	getParent() {
		return undefined;
	}

	async getChildren(): Promise<vscode.TreeItem[]> {
		if (this.treeItem) {
			return await this.treeItem.getChildren();
		} else {
			return [];
		}
	}
}

const createSourcemapPathsMap = (
	map: Map<string, Array<SourcemapNode>>,
	node: SourcemapNode
) => {
	if (node.filePaths) {
		for (const filePath of node.filePaths) {
			const existing = map.get(filePath);
			if (existing) {
				existing.push(node);
			} else {
				const created = new Array();
				created.push(node);
				map.set(filePath, created);
			}
		}
	}
	if (node.children) {
		for (const child of node.children) {
			createSourcemapPathsMap(map, child);
		}
	}
};

const createSourcemapParentsMap = (
	map: Map<SourcemapNode, SourcemapNode>,
	node: SourcemapNode,
	parent: SourcemapNode | undefined
) => {
	if (parent) {
		map.set(node, parent);
	}
	if (node.children) {
		for (const child of node.children) {
			createSourcemapParentsMap(map, child, node);
		}
	}
};
