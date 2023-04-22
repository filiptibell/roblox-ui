import * as fs from "fs";
import * as vscode from "vscode";

import { RojoTreeItem, RojoTreeProvider } from "./provider";
import { parseSourcemap } from "./utils/sourcemap";

const workspaceDestructors: Map<string, Function> = new Map();
const workspaceUpdaters: Map<string, Function> = new Map();

const disconnectWorkspace = (folder: vscode.WorkspaceFolder) => {
	const destroy = workspaceDestructors.get(folder.uri.fsPath);
	if (destroy) {
		destroy();
	}
};

const updateWorkspace = (folder: vscode.WorkspaceFolder) => {
	const update = workspaceUpdaters.get(folder.uri.fsPath);
	if (update) {
		update();
	}
};

const connectWorkspace = (
	folder: vscode.WorkspaceFolder,
	treeDataProvider: RojoTreeProvider
) => {
	// Watch the sourcemap.json in this workspace folder
	const workspacePath = folder.uri.fsPath;
	const sourcemapPath = `${workspacePath}/sourcemap.json`;
	const fileWatcher = vscode.workspace.createFileSystemWatcher(sourcemapPath);

	// Create callback for updating sourcemap
	const update = () => {
		fs.readFile(
			sourcemapPath,
			{
				encoding: "utf8",
			},
			(err, txt) => {
				if (!err) {
					const sourcemap = parseSourcemap(txt);
					treeDataProvider.update(workspacePath, sourcemap);
				}
			}
		);
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

export function activate(context: vscode.ExtensionContext) {
	// Create the main tree view and data provider
	const treeDataProvider = new RojoTreeProvider();
	const treeView = vscode.window.createTreeView("rojoExplorer", {
		treeDataProvider,
	});
	context.subscriptions.push(treeView);

	// Register global commands
	context.subscriptions.push(
		vscode.commands.registerCommand("rojoExplorer.refresh", () => {
			const workspaceFolders = vscode.workspace.workspaceFolders;
			if (workspaceFolders) {
				workspaceFolders.forEach(updateWorkspace);
			}
		})
	);

	// Register per-file commands
	context.subscriptions.push(
		vscode.commands.registerCommand(
			"rojoExplorer.openProjectRoot",
			(item: RojoTreeItem) => {
				item.openFile();
			}
		)
	);

	// Listen for focus changing to sync selection with our tree view items
	context.subscriptions.push(
		vscode.window.onDidChangeActiveTextEditor((e) => {
			if (e) {
				const filePath = e.document.uri.fsPath;
				const fileItem = treeDataProvider.find(filePath);
				if (fileItem) {
					treeView.reveal(fileItem);
				}
			}
		})
	);

	// Listen for workspace folders changing, and initialize current workspace folders
	context.subscriptions.push(
		vscode.workspace.onDidChangeWorkspaceFolders((event) => {
			for (const addedFolder of event.added) {
				connectWorkspace(addedFolder, treeDataProvider);
			}
			for (const removedFolder of event.removed) {
				disconnectWorkspace(removedFolder);
			}
		})
	);
	if (vscode.workspace.workspaceFolders) {
		vscode.workspace.workspaceFolders.forEach((folder) => {
			connectWorkspace(folder, treeDataProvider);
		});
	}
}

export function deactivate() {
	let workspacePaths = [...workspaceDestructors.keys()];
	workspacePaths.forEach((workspacePath) => {
		const destroy = workspaceDestructors.get(workspacePath);
		if (destroy) {
			destroy();
		}
	});
}
