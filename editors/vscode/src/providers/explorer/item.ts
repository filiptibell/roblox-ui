import * as vscode from "vscode";
import * as path from "path";

import {
	SourcemapNode,
	areSourcemapNodesEqual,
	findPrimaryFilePath,
	getSourcemapNodeTreeOrder,
} from "../../utils/sourcemap";

import { RojoTreeRoot } from "./root";
import { getNodeItemProps } from "./props";
import { getNullProps } from "./props";

const LOADING_ICON = new vscode.ThemeIcon("loading~spin");

export class RojoTreeItem extends vscode.TreeItem {
	private order: number | undefined;
	private node: SourcemapNode | undefined;
	private children: RojoTreeItem[] | undefined;
	private childrenDesired: boolean = false;

	constructor(
		public readonly root: RojoTreeRoot,
		private readonly eventEmitter: vscode.EventEmitter<void | vscode.TreeItem>,
		private readonly parent: RojoTreeItem | undefined | null | void
	) {
		super("Loading");
		this.iconPath = LOADING_ICON;
	}

	/**
	 * Updates the tree item with a new sourcemap node.
	 */
	public async update(
		node: SourcemapNode | undefined,
		forced: boolean | void
	): Promise<boolean> {
		let itemChanged = false;
		let childrenChanged = false;

		if (forced || !areSourcemapNodesEqual(this.node, node)) {
			const untyped = this as any;
			const newProps = node
				? await getNodeItemProps(this.root, node, this, this.parent)
				: getNullProps();
			for (const [key, value] of Object.entries(newProps)) {
				if (untyped[key] !== value) {
					if (value === null) {
						untyped[key] = undefined;
					} else {
						untyped[key] = value;
					}
					itemChanged = true;
				}
			}
		}

		const previousChildren = this.node?.children;
		const currentChildren = node?.children;
		if (previousChildren && currentChildren) {
			// Children were added, changed, and/or removed
			if (this.childrenDesired) {
				let children = this.children;
				if (!children) {
					children = [];
					this.children = children;
				}
				// Check for children being removed
				if (previousChildren.length > currentChildren.length) {
					for (
						let index = previousChildren.length;
						index > currentChildren.length;
						index--
					) {
						children.splice(index - 1, 1);
					}
					childrenChanged = true;
				}
				// Check for children being changed or added
				const promises = [];
				for (const [index, childNode] of currentChildren.entries()) {
					const childItem = children[index];
					if (childItem) {
						// Child may have changed, update it
						promises.push(childItem.update(childNode, forced));
					} else {
						// Child was added, create and update it
						const newItem = new RojoTreeItem(
							this.root,
							this.eventEmitter,
							this
						);
						promises.push(newItem.update(childNode, forced));
						children.push(newItem);
						childrenChanged = true;
					}
				}
				await Promise.all(promises);
			}
		} else if (previousChildren) {
			// All children were removed
			this.children = [];
			childrenChanged = true;
		} else if (currentChildren) {
			// All children were added
			if (this.childrenDesired) {
				const promises = [];
				const items = [];
				for (const child of currentChildren) {
					const item = new RojoTreeItem(
						this.root,
						this.eventEmitter,
						this
					);
					items.push(item);
					promises.push(item.update(child, forced));
				}
				await Promise.all(promises);
				this.children = items;
				childrenChanged = true;
			}
		}

		if (itemChanged) {
			this.order = node
				? getSourcemapNodeTreeOrder(node, this.root.metadataProvider) ??
				  undefined
				: undefined;
		}
		if (childrenChanged || !this.childrenDesired) {
			const newCollapsibleState =
				currentChildren !== undefined && currentChildren.length > 0
					? this.collapsibleState ===
					  vscode.TreeItemCollapsibleState.Expanded
						? vscode.TreeItemCollapsibleState.Expanded
						: vscode.TreeItemCollapsibleState.Collapsed
					: vscode.TreeItemCollapsibleState.None;
			if (this.collapsibleState !== newCollapsibleState) {
				this.collapsibleState = newCollapsibleState;
				childrenChanged = true;
				itemChanged = true;
			}
		}
		if (itemChanged || childrenChanged) {
			this.eventEmitter.fire(this);
		}

		this.node = node;

		return childrenChanged;
	}

	/**
	 * Opens the file path associated with this tree item.
	 *
	 * @returns `true` if the file was opened, `false` otherwise.
	 */
	public openFile(): boolean {
		const filePath = this.getFilePath();
		if (filePath) {
			vscode.commands.executeCommand(
				"vscode.open",
				vscode.Uri.file(filePath)
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
		const filePath = this.node ? findPrimaryFilePath(this.node) : null;
		return filePath ? path.join(this.root.workspacePath, filePath) : null;
	}

	/**
	 * Gets the folder path associated with this tree item.
	 */
	public getFolderPath(): string | null {
		const folderPath = this.node?.folderPath;
		return folderPath
			? path.join(this.root.workspacePath, folderPath)
			: null;
	}

	/**
	 * Gets the parent tree item for this tree item, if any.
	 */
	public getParent(): RojoTreeItem | null {
		return this.parent ? this.parent : null;
	}

	/**
	 * Gets the sourcemap node for this tree item, if any.
	 *
	 * WARNING: Modifying this sourcemap node can have unintended side effects.
	 */
	public getNode(): SourcemapNode | null {
		return this.node ? this.node : null;
	}

	/**
	 * Gets the explorer order for this tree item, if any.
	 */
	public getOrder(): number | null {
		return this.order ? this.order : null;
	}

	/**
	 * Gets a list of all child tree items for this tree item.
	 *
	 * This list of children is unique and can be mutated freely.
	 */
	public async getChildren(): Promise<RojoTreeItem[]> {
		if (this.node) {
			this.childrenDesired = true;
			if (!this.children) {
				await this.update(this.node);
			}
			const children = this.children ? [...this.children] : [];
			return children.sort(treeItemSortFunction);
		} else {
			return [];
		}
	}

	/**
	 * Clears the list of currently cached child tree items for this tree item.
	 *
	 * This may help improve performance when updating a large explorer tree.
	 */
	public async clearChildren() {
		this.childrenDesired = false;
		this.children = undefined;
		await this.update(this.node);
	}
}

const treeItemSortFunction = (left: RojoTreeItem, right: RojoTreeItem) => {
	const orderLeft = left.getOrder();
	const orderRight = right.getOrder();
	if (orderLeft !== null && orderRight !== null) {
		if (orderLeft !== orderRight) {
			return orderLeft - orderRight;
		}
	}
	const labelLeft = left.label?.toString();
	const labelRight = right.label?.toString();
	return labelLeft && labelRight ? labelLeft.localeCompare(labelRight) : 0;
};
