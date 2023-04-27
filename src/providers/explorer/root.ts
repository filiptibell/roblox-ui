import * as vscode from "vscode";

import { SourcemapNode } from "../../utils/sourcemap";
import { RobloxApiDump, RobloxReflectionMetadata } from "../../web/roblox";
import { SettingsProvider } from "../settings";

import { RojoTreeProvider } from "./tree";
import { RojoTreeItem } from "./item";
import {
	TreeItemPropChanges,
	getNullProps,
	getErroredProps,
	getLoadingProps,
} from "./props";

export class RojoTreeRoot extends vscode.TreeItem implements vscode.Disposable {
	private isLoading: boolean = true;

	private errorMessage: string | undefined;
	private loadingPath: string | undefined;
	private treeItem: RojoTreeItem | undefined;

	private sourcemap: SourcemapNode | undefined;
	private sourcemapChangePending: boolean = false;

	dispose() {}

	constructor(
		public readonly workspacePath: string,
		public readonly settingsProvider: SettingsProvider,
		public readonly treeProvider: RojoTreeProvider,
		public readonly apiDump: RobloxApiDump,
		public readonly reflectionMetadata: RobloxReflectionMetadata,
		private readonly eventEmitter: vscode.EventEmitter<void | vscode.TreeItem>
	) {
		super("<<<ROOT>>>");
		this.id = workspacePath;
		this.resourceUri = vscode.Uri.file(workspacePath);
		this.refreshTreeItem();
	}

	private async refreshTreeItem() {
		let newProps: TreeItemPropChanges = {};
		let changed = false;
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
					if (treeItem) {
						if (await treeItem.update(this.sourcemap)) {
							changed = true;
						}
					} else {
						treeItem = new RojoTreeItem(this, this.eventEmitter);
						await treeItem.update(this.sourcemap);
						this.treeItem = treeItem;
					}
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
					this.loadingPath
				);
				if (loadingProps.iconPath) {
					newProps.iconPath = loadingProps.iconPath;
				}
			}
		} else if (this.isLoading) {
			newProps = getLoadingProps(this.workspacePath, this.loadingPath);
		}
		for (const [key, value] of Object.entries(newProps)) {
			const untyped = this as any;
			if (untyped[key] !== value) {
				untyped[key] = value;
				changed = true;
			}
		}
		if (changed) {
			this.eventEmitter.fire(this);
		}
	}

	public async setLoading(loadingPath: string | undefined) {
		if (!this.isLoading || this.loadingPath !== loadingPath) {
			this.isLoading = true;
			this.loadingPath = loadingPath;
			this.clearError();
			await this.refreshTreeItem();
		}
	}

	public async clearLoading() {
		if (this.isLoading) {
			this.isLoading = false;
			this.loadingPath = undefined;
			await this.refreshTreeItem();
		}
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

	public async updateTree(rootNode: SourcemapNode) {
		this.isLoading = false;
		this.errorMessage = undefined;
		this.sourcemap = rootNode;
		this.sourcemapChangePending = true;
		await this.refreshTreeItem();
	}

	public async clearTree() {
		if (this.treeItem) {
			this.treeItem = undefined;
			this.sourcemap = undefined;
			this.sourcemapChangePending = true;
			await this.refreshTreeItem();
		}
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
