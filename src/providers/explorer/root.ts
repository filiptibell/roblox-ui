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
	private filePaths: Map<string, RojoTreeItem> = new Map();

	private errorMessage: string | undefined;
	private sourcemap: SourcemapNode | undefined;
	private treeItem: RojoTreeItem | undefined;

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
		this.updateInternalData();
	}

	private updateInternalData() {
		let newProps: TreeItemPropChanges = {};
		if (this.errorMessage) {
			newProps = getErroredProps(this.workspacePath, this.errorMessage);
		} else if (this.sourcemap) {
			newProps = getNullProps();
			// TODO: Granular tree updates
			const treeItem = new RojoTreeItem(this, this.sourcemap);
			this.filePaths = treeItem.gatherFilePaths();
			this.treeItem = treeItem;
			for (const key of Object.keys(newProps)) {
				const untyped = newProps as any;
				const value = (treeItem as any)[key];
				if (value !== undefined && value !== untyped[key]) {
					untyped[key] = value;
				}
			}
			if (this.isLoading) {
				const loadingProps = getLoadingProps(this.workspacePath);
				if (loadingProps.iconPath) {
					newProps.iconPath = loadingProps.iconPath;
				}
			}
		} else if (this.isLoading) {
			newProps = getLoadingProps(this.workspacePath);
		}
		let changed = false;
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

	public setLoading() {
		if (!this.isLoading) {
			this.isLoading = true;
			this.errorMessage = undefined;
			this.treeItem = undefined;
			this.updateInternalData();
		}
	}

	public clearLoading() {
		if (this.isLoading) {
			this.isLoading = false;
			this.updateInternalData();
		}
	}

	public setError(errorMessage: string) {
		if (this.errorMessage !== errorMessage) {
			this.errorMessage = errorMessage;
			this.isLoading = false;
			this.treeItem = undefined;
			this.updateInternalData();
		}
	}

	public clearError() {
		if (this.errorMessage) {
			this.errorMessage = undefined;
			this.updateInternalData();
		}
	}

	public update(rootNode: SourcemapNode) {
		this.isLoading = false;
		this.errorMessage = undefined;
		this.sourcemap = rootNode;
		this.updateInternalData();
	}

	public find(filePath: string): RojoTreeItem | null {
		const treeItem = this.filePaths.get(filePath);
		if (treeItem) {
			return treeItem;
		} else {
			return null;
		}
	}

	getTreeItem() {
		return undefined;
	}

	getParent() {
		return undefined;
	}

	getChildren(): vscode.TreeItem[] {
		if (this.treeItem) {
			return this.treeItem.getChildren();
		} else {
			return [];
		}
	}
}
