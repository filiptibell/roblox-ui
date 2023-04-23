import * as vscode from "vscode";

import { RojoTreeItem, RojoTreeProvider } from "./provider";
import { SettingsManager } from "./utils/settings";
import {
	connectAllWorkspaces,
	connectWorkspace,
	disconnectAllWorkspaces,
	disconnectWorkspace,
	updateAllWorkspaces,
} from "./workspaces";

export function activate(context: vscode.ExtensionContext) {
	// Create the main tree view and data provider
	const treeDataProvider = new RojoTreeProvider();
	const treeView = vscode.window.createTreeView("rojoExplorer", {
		treeDataProvider,
	});
	context.subscriptions.push(treeView);

	// Register global commands
	context.subscriptions.push(
		vscode.commands.registerCommand(
			"rojoExplorer.refresh",
			updateAllWorkspaces
		)
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

	// Listen for settings changing, if any of the settings that
	// change behavior of the sourcemap or the sourcemap watch
	// command change we have to re-initialize the workspace
	const settingsManager = new SettingsManager();
	settingsManager.listen("includeNonScripts", (value) => {
		connectAllWorkspaces(settingsManager, treeDataProvider);
	});
	settingsManager.listen("rojoProjectFile", () => {
		connectAllWorkspaces(settingsManager, treeDataProvider);
	});
	context.subscriptions.push(settingsManager);

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
				connectWorkspace(
					addedFolder,
					settingsManager,
					treeDataProvider
				);
			}
			for (const removedFolder of event.removed) {
				disconnectWorkspace(removedFolder);
			}
		})
	);
	connectAllWorkspaces(settingsManager, treeDataProvider);
}

export function deactivate() {
	disconnectAllWorkspaces();
}
