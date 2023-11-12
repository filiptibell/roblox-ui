import * as vscode from "vscode";

import { RojoTreeProvider } from "./providers/explorer";
import { SettingsProvider } from "./providers/settings";
import { connectSourcemapUsingServer } from "./utils/sourcemap";

const workspaceRefreshers: Map<string, () => boolean> = new Map();
const workspaceReloaders: Map<string, () => void> = new Map();
const workspaceDestructors: Map<string, () => Promise<void>> = new Map();

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

export const connectAllWorkspaces = async (
	context: vscode.ExtensionContext,
	settings: SettingsProvider,
	provider: RojoTreeProvider
) => {
	let promisesDisconnect = new Array<Promise<void>>();
	let promisesConnect = new Array<Promise<void>>();

	if (vscode.workspace.workspaceFolders) {
		vscode.workspace.workspaceFolders.forEach((folder) => {
			promisesDisconnect.push(disconnectWorkspace(folder));
		});
		vscode.workspace.workspaceFolders.forEach((folder) => {
			promisesConnect.push(
				connectWorkspace(context, folder, settings, provider)
			);
		});
	}

	await Promise.all(promisesDisconnect);
	await Promise.all(promisesConnect);
};

export const disconnectWorkspace = async (folder: vscode.WorkspaceFolder) => {
	const destroy = workspaceDestructors.get(folder.uri.fsPath);
	if (destroy) {
		await destroy();
	}
};

export const disconnectAllWorkspaces = async () => {
	let workspacePaths = [...workspaceDestructors.keys()];
	let workspacePromises = new Array<Promise<void>>();

	workspacePaths.forEach((workspacePath) => {
		const destroy = workspaceDestructors.get(workspacePath);
		if (destroy) {
			workspacePromises.push(destroy());
		}
	});

	await Promise.all(workspacePromises);
};
