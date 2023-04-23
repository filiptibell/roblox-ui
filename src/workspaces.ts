import * as fs from "fs/promises";
import * as vscode from "vscode";

import { RojoTreeProvider } from "./provider";
import { parseSourcemap } from "./utils/sourcemap";

const workspaceDestructors: Map<string, Function> = new Map();
const workspaceUpdaters: Map<string, Function> = new Map();

export const updateWorkspace = (folder: vscode.WorkspaceFolder) => {
	const update = workspaceUpdaters.get(folder.uri.fsPath);
	if (update) {
		update();
	}
};

export const updateAllWorkspaces = () => {
	const workspaceFolders = vscode.workspace.workspaceFolders;
	if (workspaceFolders) {
		workspaceFolders.forEach(updateWorkspace);
	}
};

export const connectWorkspace = (
	folder: vscode.WorkspaceFolder,
	treeDataProvider: RojoTreeProvider
) => {
	// Watch the sourcemap.json in this workspace folder
	const workspacePath = folder.uri.fsPath;
	const sourcemapPath = `${workspacePath}/sourcemap.json`;
	const fileWatcher = vscode.workspace.createFileSystemWatcher(sourcemapPath);

	// Create callback for updating sourcemap
	const update = async () => {
		const sourcemapJson = await fs.readFile(sourcemapPath, "utf8");
		treeDataProvider.update(workspacePath, parseSourcemap(sourcemapJson));
	};

	// Create callback for disconnecting (destroying)
	// everything created for this workspace folder
	const destroy = () => {
		workspaceUpdaters.delete(workspacePath);
		workspaceDestructors.delete(workspacePath);
		treeDataProvider.delete(workspacePath);
		fileWatcher.dispose();
	};

	// Store callbacks to access them from other listeners
	workspaceUpdaters.set(workspacePath, update);
	workspaceDestructors.set(workspacePath, destroy);

	// Start watching the sourcemap for changes and update once initially
	fileWatcher.onDidChange(update);
	updateWorkspace(folder);
};

export const connectAllWorkspaces = (provider: RojoTreeProvider) => {
	if (vscode.workspace.workspaceFolders) {
		vscode.workspace.workspaceFolders.forEach((folder) => {
			disconnectWorkspace(folder);
		});
		vscode.workspace.workspaceFolders.forEach((folder) => {
			connectWorkspace(folder, provider);
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
