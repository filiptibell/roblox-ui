import * as vscode from "vscode";

import { RojoTreeProvider } from "./providers/explorer";
import { SettingsProvider } from "./providers/settings";
import { rojoSupportsSourcemapWatch } from "./utils/rojo";
import {
	connectSourcemapUsingFile,
	connectSourcemapUsingRojo,
} from "./utils/sourcemap";

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
	folder: vscode.WorkspaceFolder,
	settings: SettingsProvider,
	treeProvider: RojoTreeProvider
) => {
	const workspacePath = folder.uri.fsPath;

	// Check for autogeneration setting, and ensure sourcemap
	// with watching is supported if we want to autogenerate
	let autogenerate = settings.get("sourcemap.autogenerate");
	if (autogenerate) {
		const supported = await rojoSupportsSourcemapWatch(workspacePath);
		if (!supported) {
			autogenerate = false;
		}
	}

	if (autogenerate) {
		// Autogeneration is enabled and available, we can
		// watch for changes using the rojo sourcemap cli command
		const [refresh, reload, destroy] = connectSourcemapUsingRojo(
			workspacePath,
			settings,
			treeProvider
		);
		workspaceRefreshers.set(workspacePath, refresh);
		workspaceReloaders.set(workspacePath, reload);
		workspaceDestructors.set(workspacePath, destroy);
	} else {
		// Autogeneration is either disabled or not available, so we will
		// instead watch the sourcemap.json file in this workspace folder
		const [refresh, reload, destroy] = connectSourcemapUsingFile(
			workspacePath,
			settings,
			treeProvider
		);
		workspaceRefreshers.set(workspacePath, refresh);
		workspaceReloaders.set(workspacePath, reload);
		workspaceDestructors.set(workspacePath, destroy);
	}
};

export const connectAllWorkspaces = (
	settings: SettingsProvider,
	provider: RojoTreeProvider
) => {
	if (vscode.workspace.workspaceFolders) {
		vscode.workspace.workspaceFolders.forEach((folder) => {
			disconnectWorkspace(folder);
		});
		vscode.workspace.workspaceFolders.forEach((folder) => {
			connectWorkspace(folder, settings, provider);
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
