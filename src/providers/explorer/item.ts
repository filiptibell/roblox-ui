import * as vscode from "vscode";
import * as path from "path";

import {
	SourcemapNode,
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
	public async update(node: SourcemapNode | undefined): Promise<boolean> {
		let itemChanged = false;
		let childrenChanged = false;

		if (nodesAreDifferent(this.node, node)) {
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
		if (previousChildren && currentChildren && this.children) {
			// Check for children being removed
			if (previousChildren.length > currentChildren.length) {
				for (
					let index = previousChildren.length;
					index > currentChildren.length;
					index--
				) {
					this.children.splice(index - 1, 1);
				}
				childrenChanged = true;
			}
			// Check for children being changed or added
			const added = [];
			const promises = [];
			for (const [index, childNode] of currentChildren.entries()) {
				const childItem = this.children[index];
				if (childItem) {
					// Child may have changed, update it
					promises.push(childItem.update(childNode));
				} else {
					// Child was added, create and update it
					const newItem = new RojoTreeItem(
						this.root,
						this.eventEmitter,
						this
					);
					added.push(newItem);
					promises.push(newItem.update(childNode));
				}
			}
			await Promise.all(promises);
			if (added.length > 0) {
				for (const child of added.values()) {
					this.children.push(child);
				}
				childrenChanged = true;
			}
		} else if (currentChildren && !this.children) {
			// No children yet, create initial children
			const promises = [];
			const items = [];
			if (currentChildren) {
				for (const child of currentChildren) {
					const item = new RojoTreeItem(
						this.root,
						this.eventEmitter,
						this
					);
					items.push(item);
					promises.push(item.update(child));
				}
			}
			await Promise.all(promises);
			this.children = items;
			childrenChanged = true;
		} else if (previousChildren) {
			// All children were removed
			this.children = [];
			childrenChanged = true;
		}

		if (itemChanged) {
			this.order = node
				? getSourcemapNodeTreeOrder(
						node,
						this.root.reflectionMetadata
				  ) ?? undefined
				: undefined;
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
			if (!this.children) {
				await this.update(this.node);
			}
			const children = this.children ? [...this.children] : [];
			return children.sort(treeItemSortFunction);
		} else {
			return [];
		}
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

const nodesAreDifferent = (
	previous: SourcemapNode | undefined,
	current: SourcemapNode | undefined
): boolean => {
	const propChanged =
		previous?.folderPath !== current?.folderPath ||
		previous?.className !== current?.className ||
		previous?.name !== current?.name;
	// TODO: Check file paths
	return propChanged;
};
