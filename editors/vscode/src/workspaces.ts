import * as vscode from "vscode";

import { SettingsProvider } from "./providers/settings";
import { ExplorerTreeProvider } from "./explorer";
import { RpcServer } from "./server";

const workspaceServers: Map<string, RpcServer> = new Map();

let currentContext: vscode.ExtensionContext;
let currentSettings: SettingsProvider;
let currentProvider: ExplorerTreeProvider;

export const connectAllWorkspaces = async (
	context: vscode.ExtensionContext,
	settings: SettingsProvider,
	provider: ExplorerTreeProvider,
) => {
	await disconnectAllWorkspaces();

	currentContext = context;
	currentSettings = settings;
	currentProvider = provider;

	const promises = new Array<Promise<void>>();

	if (vscode.workspace.workspaceFolders) {
		for (const workspaceFolder of vscode.workspace.workspaceFolders) {
			const workspacePath = workspaceFolder.uri.fsPath;
			const workspaceServer = new RpcServer(context, workspacePath, settings);
			workspaceServers.set(workspacePath, workspaceServer);
			provider.connectServer(workspacePath, workspaceServer);
		}
	}

	await Promise.all(promises);
};

export const disconnectAllWorkspaces = async () => {
	if (currentProvider) {
		currentProvider.disconnectAllServers();
	}

	const promises = new Array<Promise<void>>();

	for (const [_, server] of workspaceServers) {
		promises.push(server.stop());
	}
	workspaceServers.clear();

	await Promise.all(promises);
};

export const reconnectAllWorkspaces = async () => {
	if (currentContext && currentSettings && currentProvider) {
		await connectAllWorkspaces(currentContext, currentSettings, currentProvider);
	}
};
