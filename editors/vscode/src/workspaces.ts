import * as vscode from "vscode";

import { RojoTreeProvider } from "./providers/explorer";
import { SettingsProvider } from "./providers/settings";
import { connectSourcemapUsingServer } from "./utils/sourcemap";

const workspaceRefreshers: Map<string, Function> = new Map();
const workspaceReloaders: Map<string, Function> = new Map();
const workspaceDestructors: Map<string, Function> = new Map();

export const refreshWorkspace = (folder: vscode.WorkspaceFolder) => {
	const update = workspaceRefreshers.get(folder.uri.fsPath);
	if (update) {
		update();
	}
};

export const refreshAllWorkspaces = () => {
	const workspaceFolders = vscode.workspace.workspaceFolders;
	if (workspaceFolders) {
		workspaceFolders.forEach(refreshWorkspace);
	}
};

export const reloadWorkspace = (folder: vscode.WorkspaceFolder) => {
	const update = workspaceReloaders.get(folder.uri.fsPath);
	if (update) {
		update();
	}
};

export const reloadAllWorkspaces = () => {
	const workspaceFolders = vscode.workspace.workspaceFolders;
	if (workspaceFolders) {
		workspaceFolders.forEach(reloadWorkspace);
	}
};

export const connectWorkspace = async (
	context: vscode.ExtensionContext,
	folder: vscode.WorkspaceFolder,
	settings: SettingsProvider,
	treeProvider: RojoTreeProvider
) => {
	const workspacePath = folder.uri.fsPath;

	const [refresh, reload, destroy] = connectSourcemapUsingServer(
		context,
		workspacePath,
		settings,
		treeProvider
	);

	workspaceRefreshers.set(workspacePath, refresh);
	workspaceReloaders.set(workspacePath, reload);
	workspaceDestructors.set(workspacePath, destroy);
};

export const connectAllWorkspaces = (
	context: vscode.ExtensionContext,
	settings: SettingsProvider,
	provider: RojoTreeProvider
) => {
	if (vscode.workspace.workspaceFolders) {
		vscode.workspace.workspaceFolders.forEach((folder) => {
			disconnectWorkspace(folder);
		});
		vscode.workspace.workspaceFolders.forEach((folder) => {
			connectWorkspace(context, folder, settings, provider);
		});
	}
};

export const disconnectWorkspace = (folder: vscode.WorkspaceFolder) => {
	const destroy = workspaceDestructors.get(folder.uri.fsPath);
	if (destroy) {
		destroy();
	}
};

export const disconnectAllWorkspaces = () => {
	let workspacePaths = [...workspaceDestructors.keys()];
	workspacePaths.forEach((workspacePath) => {
		const destroy = workspaceDestructors.get(workspacePath);
		if (destroy) {
			destroy();
		}
	});
};
