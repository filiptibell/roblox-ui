import * as vscode from "vscode";

import { SettingsManager } from "./utils/settings";
import {
	deleteExistingInstance,
	promptNewInstanceCreation,
	promptRenameExistingInstance,
} from "./utils/instances";

import {
	connectAllWorkspaces,
	connectWorkspace,
	disconnectAllWorkspaces,
	disconnectWorkspace,
	updateAllWorkspaces,
} from "./workspaces";

import { connectSelection, disconnectSelection } from "./selection";

import { RojoTreeItem, RojoTreeProvider } from "./provider";

import {
	getRobloxApiDump,
	getRobloxApiReflection,
	getRobloxApiVersion,
} from "./web/roblox";

export async function activate(context: vscode.ExtensionContext) {
	// Fetch api dump and reflection metadata, if the user does not
	// have an internet connection the very first time they activate
	// the extension this may fail but will otherwise fall back to a
	// cached version and warn the user about the potential desync
	let apiVersion;
	let apiDump;
	let apiReflection;
	try {
		apiVersion = await getRobloxApiVersion(context);
		apiDump = await getRobloxApiDump(context, apiVersion);
		apiReflection = await getRobloxApiReflection(context, apiVersion);
	} catch (err) {
		vscode.window.showErrorMessage(`${err}`);
		return;
	}

	// Create the main tree view and data provider
	const treeDataProvider = new RojoTreeProvider(apiDump, apiReflection);
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
			"rojoExplorer.insertObject",
			(item: RojoTreeItem) => {
				promptNewInstanceCreation(
					item.getFolderPath(),
					item.getFilePath()
				);
			}
		)
	);
	context.subscriptions.push(
		vscode.commands.registerCommand(
			"rojoExplorer.renameObject",
			(item: RojoTreeItem) => {
				promptRenameExistingInstance(
					item.getFolderPath(),
					item.getFilePath()
				);
			}
		)
	);
	context.subscriptions.push(
		vscode.commands.registerCommand(
			"rojoExplorer.deleteObject",
			(item: RojoTreeItem) => {
				deleteExistingInstance(
					item.getFolderPath(),
					item.getFilePath()
				);
			}
		)
	);
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
	settingsManager.listen("includeNonScripts", () => {
		connectAllWorkspaces(settingsManager, treeDataProvider);
	});
	settingsManager.listen("rojoProjectFile", () => {
		connectAllWorkspaces(settingsManager, treeDataProvider);
	});
	context.subscriptions.push(settingsManager);

	// Listen for focus changing to sync selection with our tree view items
	context.subscriptions.push(
		vscode.window.onDidChangeActiveTextEditor((e) => {
			if (e && treeView.visible) {
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
	connectSelection(treeView, treeDataProvider);
}

export function deactivate() {
	disconnectAllWorkspaces();
	disconnectSelection();
}
