import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";

export type SourcemapNode = {
	name: string;
	className: string;
	filePaths?: string[];
	children?: SourcemapNode[];
};

const getTreeItemOrder = (node: SourcemapNode): number => {
	if (
		node.className == "Script" ||
		node.className == "LocalScript" ||
		node.className == "ModuleScript"
	) {
		return 2;
	} else {
		return 1;
	}
};

export class RojoTreeProvider implements vscode.TreeDataProvider<RojoTreeItem> {
	private roots: Map<string, SourcemapNode> = new Map();

	private _onDidChangeTreeData: vscode.EventEmitter<
		RojoTreeItem | undefined | null | void
	> = new vscode.EventEmitter<RojoTreeItem | undefined | null | void>();
	readonly onDidChangeTreeData: vscode.Event<
		RojoTreeItem | undefined | null | void
	> = this._onDidChangeTreeData.event;

	constructor(private context: vscode.ExtensionContext) {
		context.subscriptions.push(
			vscode.commands.registerCommand(
				"rojoExplorer.openProjectRoot",
				(item: RojoTreeItem) => {
					item.openFile();
				}
			)
		);
	}

	public update(workspacePath: string, rootNode: SourcemapNode) {
		this.roots.set(workspacePath, rootNode);
		this._onDidChangeTreeData.fire();
	}

	public delete(workspacePath: string) {
		this.roots.delete(workspacePath);
		this._onDidChangeTreeData.fire();
	}

	getTreeItem(item: RojoTreeItem): vscode.TreeItem {
		return item;
	}

	getChildren(item?: RojoTreeItem): Thenable<RojoTreeItem[] | null> {
		if (!item) {
			const workspacePaths = [...this.roots.keys()];
			workspacePaths.sort();
			return Promise.resolve(
				workspacePaths.map(
					(workspacePath) =>
						new RojoTreeItem(
							workspacePath,
							this.roots.get(workspacePath)!
						)
				)
			);
		} else {
			const children = item.getChildren();
			children.sort((left, right) => {
				const leftOrder = getTreeItemOrder(left);
				const rightOrder = getTreeItemOrder(right);
				if (leftOrder != rightOrder) {
					return leftOrder - rightOrder;
				} else {
					return left.name.localeCompare(right.name);
				}
			});
			return Promise.resolve(
				children.map(
					(child) => new RojoTreeItem(item.workspaceRoot, child)
				)
			);
		}
	}
}

class RojoTreeItem extends vscode.TreeItem {
	private filePath: string | null = null;
	private fileIsScript: boolean = false;

	constructor(
		public readonly workspaceRoot: string,
		private node: SourcemapNode
	) {
		super("<<<INSTANCE>>>");
		this.update(node);
	}

	public update(node: SourcemapNode) {
		this.node = node;

		this.label = node.name;
		this.collapsibleState = node.children
			? vscode.TreeItemCollapsibleState.Collapsed
			: vscode.TreeItemCollapsibleState.None;

		this.tooltip = node.className;
		this.iconPath = path.join(
			__dirname,
			"..",
			"icons",
			`${node.className}.png`
		);

		if (node.filePaths) {
			let filePath = node.filePaths.find((filePath) => {
				if (
					filePath.endsWith(".lua") ||
					filePath.endsWith(".luau") ||
					filePath.endsWith(".project.json")
				) {
					return filePath;
				}
			});
			if (filePath) {
				this.filePath = path.join(this.workspaceRoot, filePath);
				this.fileIsScript = !filePath.endsWith(".project.json");
			}
		}

		if (this.filePath && this.fileIsScript) {
			this.command = {
				title: "Open file",
				command: "vscode.open",
				arguments: [vscode.Uri.file(this.filePath)],
			};
		} else if (this.filePath) {
			this.contextValue = "projectRoot";
		}
	}

	public openFile() {
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

	public getChildren(): SourcemapNode[] {
		if (this.node.children) {
			return [...this.node.children];
		} else {
			return [];
		}
	}
}
