import * as vscode from "vscode";

import { DomFindByQueryRequest, DomInstance, RpcServer } from "../../server";

import { ExplorerItem, compareExplorerItemOrder } from "./item";
import { Providers } from "..";

export * from "./item";

export class ExplorerTreeProvider implements vscode.TreeDataProvider<ExplorerItem> {
	private readonly loaded: Map<string, boolean> = new Map();
	private readonly servers: Map<string, RpcServer> = new Map();
	private readonly disconnects: Map<string, () => boolean> = new Map();
	private readonly explorerRoots: Map<string, ExplorerItem | null> = new Map();
	private readonly explorerIdMaps: Map<string, Map<string, ExplorerItem>> = new Map();

	private readonly _onDidChangeTreeData: vscode.EventEmitter<ExplorerItem | undefined | null> =
		new vscode.EventEmitter();
	public readonly onDidChangeTreeData: vscode.Event<ExplorerItem | undefined | null> =
		this._onDidChangeTreeData.event;

	private readonly intervalHandle: NodeJS.Timeout;

	constructor(public readonly providers: Providers) {
		// HACK: It seems like sometimes startup is ... too fast ?? and we run
		// into a race condition where vscode does not show our tree properly,
		// this here forces the tree to refresh every so often while we don't
		// have any tree / root instance / data models loaded in, and fixes it
		this.intervalHandle = setInterval(() => {
			if (this.explorerRoots.size <= 0) {
				this._onDidChangeTreeData.fire(null);
			}
		}, 200);
	}

	public dispose() {
		clearInterval(this.intervalHandle);
		this.disconnectAllServers();
	}

	public async getTreeItem(item: ExplorerItem) {
		return item;
	}

	public async getChildren(parent?: ExplorerItem) {
		const items = new Array<ExplorerItem>();

		if (parent) {
			// We got an explorer item to get the children of, so request that from the server
			const idMap = this.explorerIdMaps.get(parent.workspacePath);
			if (idMap === undefined) {
				throw new Error("Missing id map");
			}
			const server = this.servers.get(parent.workspacePath);
			if (server === undefined) {
				throw new Error("Missing server");
			}
			const children = await server.sendRequest("dom/children", {
				id: parent.domInstance.id,
			});
			if (children && children.length > 0) {
				for (const childInstance of children) {
					const item = new ExplorerItem(
						this.providers,
						parent.workspacePath,
						childInstance,
						false,
						parent,
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
			this.explorerRoots.clear();
			for (const [workspacePath, server] of this.servers) {
				const rootInstance = await server.sendRequest("dom/root", null);
				if (rootInstance) {
					const item = new ExplorerItem(
						this.providers,
						workspacePath,
						rootInstance,
						true,
						null,
					);
					const idMap = this.explorerIdMaps.get(workspacePath);
					if (idMap === undefined) {
						throw new Error("Missing id map");
					}
					idMap.set(item.domInstance.id, item);
					items.push(item);
					this.explorerRoots.set(workspacePath, item);
				}
			}
		}

		items.sort(compareExplorerItemOrder);

		if (parent) {
			parent.setChildReferences(items);
		}

		return items;
	}

	public getParent(item: ExplorerItem) {
		return item.parent;
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
			if (item?.domInstance.children) {
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

	public disconnectAllServers() {
		for (const [_, disconnect] of this.disconnects) {
			disconnect();
		}

		this.explorerRoots.clear();
		this.explorerIdMaps.clear();
		this.disconnects.clear();
		this.servers.clear();
		this.loaded.clear();

		this._onDidChangeTreeData.fire(null);
	}

	public disconnectServer(workspacePath: string) {
		const server = this.servers.get(workspacePath);
		if (server !== undefined) {
			const disconnect = this.disconnects.get(workspacePath);
			if (disconnect !== undefined) {
				disconnect();
			}

			this.explorerRoots.delete(workspacePath);
			this.explorerIdMaps.delete(workspacePath);
			this.disconnects.delete(workspacePath);
			this.servers.delete(workspacePath);
			this.loaded.delete(workspacePath);
			this._onDidChangeTreeData.fire(null);
		}
	}

	public connectServer(workspacePath: string, server: RpcServer) {
		this.disconnectServer(workspacePath);
		this.explorerIdMaps.set(workspacePath, new Map());
		const disconnect = server.onRequest("dom/notification", (notif) => {
			if (notif !== null) {
				if (notif.kind === "Added") {
					if (notif.data.parentId) {
						this.refreshItemById(workspacePath, notif.data.parentId);
					}
				} else if (notif.kind === "Removed") {
					this.deleteItemById(workspacePath, notif.data.childId);
					if (notif.data.parentId) {
						this.refreshItemById(workspacePath, notif.data.parentId);
					}
				} else if (notif.kind === "Changed") {
					this.refreshItemById(workspacePath, notif.data.id);
				}
				if (this.loaded.get(workspacePath) === false) {
					this.loaded.set(workspacePath, true);
					this._onDidChangeTreeData.fire(null);
				}
			} else {
				if (this.loaded.get(workspacePath) === true) {
					this.loaded.set(workspacePath, false);
					this._onDidChangeTreeData.fire(null);
				}
			}
		});
		this.disconnects.set(workspacePath, disconnect);
		this.servers.set(workspacePath, server);
		this._onDidChangeTreeData.fire(null);
	}

	public async getAncestors(workspacePath: string, domId: string): Promise<DomInstance[] | null> {
		const server = this.servers.get(workspacePath);
		if (server) {
			const response = await server.sendRequest("dom/ancestors", {
				id: domId,
			});
			if (response) {
				return response;
			}
		}
		return null;
	}

	public async getFullName(workspacePath: string, domId: string): Promise<string[] | null> {
		const ancestors = await this.getAncestors(workspacePath, domId);
		if (ancestors) {
			const names = new Array<string>();
			for (const ancestor of ancestors) {
				names.push(ancestor.name);
			}
			return names;
		}
		return null;
	}

	public findById(workspacePath: string, domId: string): ExplorerItem | null {
		const idMap = this.explorerIdMaps.get(workspacePath);
		if (idMap) {
			const item = idMap.get(domId);
			if (item) {
				return item;
			}
		}
		return null;
	}

	public async findByPath(workspacePath: string, path: string): Promise<DomInstance | null> {
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
		query: string | DomFindByQueryRequest,
	): Promise<DomInstance[]> {
		const server = this.servers.get(workspacePath);
		if (server) {
			const response = await server.sendRequest(
				"dom/findByQuery",
				typeof query === "string" ? { query, limit: undefined } : query,
			);
			if (response && response.length > 0) {
				return response;
			}
		}
		return [];
	}

	public async revealById(workspacePath: string, domId: string, select?: true | null) {
		const ancestors = await this.getAncestors(workspacePath, domId);
		if (!ancestors) {
			return;
		}

		let foundAll = true;
		for (const [index, ancestor] of ancestors.entries()) {
			const item = this.findById(workspacePath, ancestor.id);
			if (!item) {
				foundAll = false;
				break;
			}
			if (index < ancestors.length - 1) {
				await item.expand(); // NOTE: Don't expand the last item
			}
		}

		if (foundAll && select === true && ancestors.length > 0) {
			const last = ancestors[ancestors.length - 1];
			const item = this.findById(workspacePath, last.id);
			if (item === null) {
				throw new Error("Missing item");
			}
			await item.select();
		}
	}

	public async revealByPath(path: string, select?: true | null) {
		for (const [workspacePath, _] of this.servers) {
			if (path.startsWith(workspacePath)) {
				const domInstance = await this.findByPath(workspacePath, path);
				if (domInstance) {
					await this.revealById(workspacePath, domInstance.id, select);
				}
			}
		}
	}

	public async insertInstance(
		workspacePath: string,
		parentDomId: string,
		desiredClassName: string,
		desiredName: string,
	): Promise<string | null> {
		const server = this.servers.get(workspacePath);
		if (server) {
			return await server.sendRequest("instance/insert", {
				parentId: parentDomId,
				className: desiredClassName,
				name: desiredName,
			}) ?? null;
		}
		return null;
	}

	public async renameInstance(
		workspacePath: string,
		domId: string,
		desiredName: string,
	): Promise<boolean> {
		const server = this.servers.get(workspacePath);
		if (server) {
			return await server.sendRequest("instance/rename", {
				id: domId,
				name: desiredName,
			});
		}
		return false;
	}

	public async deleteInstance(workspacePath: string, domId: string): Promise<boolean> {
		const server = this.servers.get(workspacePath);
		if (server) {
			return await server.sendRequest("instance/delete", {
				id: domId,
			});
		}
		return false;
	}

	public async moveInstance(
		workspacePath: string,
		domId: string,
		newParentId: string,
	): Promise<boolean> {
		const server = this.servers.get(workspacePath);
		if (server) {
			return await server.sendRequest("instance/move", {
				id: domId,
				parentId: newParentId,
			});
		}
		return false;
	}
}
