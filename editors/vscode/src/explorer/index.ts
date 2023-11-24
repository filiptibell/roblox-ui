import * as vscode from "vscode";

import { IconsProvider } from "../providers/icons";
import { MetadataProvider } from "../providers/metadata";
import { SettingsProvider } from "../providers/settings";

import { DomInstance, RpcServer } from "../server";

import { ExplorerItem, compareExplorerItemOrder } from "./item";

export * from "./item";

export class ExplorerTreeProvider
	implements vscode.TreeDataProvider<ExplorerItem>
{
	private loaded: Map<string, boolean> = new Map();
	private servers: Map<string, RpcServer> = new Map();
	private disconnects: Map<string, () => boolean> = new Map();
	private explorerIdMaps: Map<string, Map<string, ExplorerItem>> = new Map();

	readonly _onDidChangeTreeData: vscode.EventEmitter<void | ExplorerItem> =
		new vscode.EventEmitter();
	public readonly onDidChangeTreeData: vscode.Event<void | ExplorerItem> =
		this._onDidChangeTreeData.event;

	constructor(
		public readonly settingsProvider: SettingsProvider,
		public readonly metadataProvider: MetadataProvider,
		public readonly iconsProvider: IconsProvider
	) {}

	public dispose() {
		for (const workspacePath of this.servers.keys()) {
			this.disconnectServer(workspacePath);
		}
	}

	public async getTreeItem(item: ExplorerItem) {
		return item;
	}

	public async getChildren(parent?: ExplorerItem) {
		const items = new Array<ExplorerItem>();
		if (parent) {
			// We got an explorer item to get the children of, so request that from the server
			const idMap = this.explorerIdMaps.get(parent.workspacePath)!;
			const server = this.servers.get(parent.workspacePath)!;
			const children = await server.sendRequest("dom/children", {
				id: parent.domInstance.id,
			});
			if (children && children.length > 0) {
				for (const childInstance of children) {
					const item = new ExplorerItem(
						parent.workspacePath,
						this,
						childInstance,
						false
					);
					idMap.set(item.domInstance.id, item);
					items.push(item);
				}
			}
		} else {
			// We got no explorer item to get children of, so here
			// we fetch the root items for all known workspace paths
			// TODO: Re-implement the explorer.showDataModel setting, if not set to true
			// then we should show datamodel children directly for single-root workspaces
			for (const [workspacePath, server] of this.servers) {
				const rootInstance = await server.sendRequest("dom/root", null);
				if (rootInstance) {
					const item = new ExplorerItem(
						workspacePath,
						this,
						rootInstance,
						true
					);
					const idMap = this.explorerIdMaps.get(workspacePath)!;
					idMap.set(item.domInstance.id, item);
					items.push(item);
				}
			}
		}
		items.sort(compareExplorerItemOrder);
		return items;
	}

	public getParent(item: ExplorerItem) {
		const parentId = item.domInstance.parentId;
		if (parentId && parentId.length > 0) {
			const idMap = this.explorerIdMaps.get(item.workspacePath);
			if (idMap) {
				return idMap.get(parentId);
			}
		}
		return undefined;
	}

	private refreshItemById(workspacePath: string, id: string) {
		const idMap = this.explorerIdMaps.get(workspacePath);
		if (idMap) {
			const item = idMap.get(id);
			if (item) {
				this._onDidChangeTreeData.fire(item);
			}
		}
	}

	private deleteItemById(workspacePath: string, id: string) {
		const idMap = this.explorerIdMaps.get(workspacePath);
		if (idMap) {
			const item = idMap.get(id);
			if (item && item.domInstance.children) {
				for (const childId of item.domInstance.children) {
					this.deleteItemById(workspacePath, childId);
				}
			}
			idMap.delete(id);
		}
	}

	public getWorkspacePaths() {
		return Array.from(this.servers.keys());
	}

	public getServer(workspacePath: string) {
		return this.servers.get(workspacePath);
	}

	public disconnectServer(workspacePath: string) {
		const server = this.servers.get(workspacePath);
		if (server !== undefined) {
			const disconnect = this.disconnects.get(workspacePath);
			if (disconnect !== undefined) {
				disconnect();
			}
			this.explorerIdMaps.delete(workspacePath);
			this.disconnects.delete(workspacePath);
			this.servers.delete(workspacePath);
			this.loaded.delete(workspacePath);
			this._onDidChangeTreeData.fire();
		}
	}

	public connectServer(workspacePath: string, server: RpcServer) {
		this.disconnectServer(workspacePath);
		this.explorerIdMaps.set(workspacePath, new Map());
		const disconnect = server.onRequest("dom/notification", (notif) => {
			if (notif !== null) {
				if (notif.kind === "Added") {
					if (notif.data.parentId) {
						this.refreshItemById(
							workspacePath,
							notif.data.parentId
						);
					}
				} else if (notif.kind === "Removed") {
					this.deleteItemById(workspacePath, notif.data.childId);
					if (notif.data.parentId) {
						this.refreshItemById(
							workspacePath,
							notif.data.parentId
						);
					}
				} else if (notif.kind === "Changed") {
					this.refreshItemById(workspacePath, notif.data.id);
				}
				if (this.loaded.get(workspacePath) === false) {
					this.loaded.set(workspacePath, true);
					this._onDidChangeTreeData.fire();
				}
			} else {
				if (this.loaded.get(workspacePath) === true) {
					this.loaded.set(workspacePath, false);
					this._onDidChangeTreeData.fire();
				}
			}
		});
		this.disconnects.set(workspacePath, disconnect);
		this.servers.set(workspacePath, server);
		this.loaded.set(workspacePath, false);
		this._onDidChangeTreeData.fire();
	}

	public async findExplorerItem(
		workspacePath: string,
		domId: string
	): Promise<ExplorerItem | null> {
		const idMap = this.explorerIdMaps.get(workspacePath);
		if (idMap) {
			const item = idMap.get(domId);
			if (item) {
				return item;
			}
		}
		return null;
	}

	public async findByPath(
		workspacePath: string,
		path: string
	): Promise<DomInstance | null> {
		const server = this.servers.get(workspacePath);
		if (server) {
			const response = await server.sendRequest("dom/findByPath", {
				path,
			});
			if (response) {
				return response;
			}
		}
		return null;
	}

	public async findByQuery(
		workspacePath: string,
		query: string,
		limit?: number
	): Promise<DomInstance[]> {
		const server = this.servers.get(workspacePath);
		if (server) {
			const response = await server.sendRequest("dom/findByQuery", {
				query,
				limit,
			});
			if (response && response.length > 0) {
				return response;
			}
		}
		return [];
	}
}
