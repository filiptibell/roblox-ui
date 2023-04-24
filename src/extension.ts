import * as vscode from "vscode";

import {
	connectAllWorkspaces,
	connectWorkspace,
	disconnectAllWorkspaces,
	disconnectWorkspace,
} from "./workspaces";

import { RojoTreeProvider } from "./providers/tree";
import { SettingsProvider } from "./providers/settings";
import { SelectionProvider } from "./providers/selection";
import { CommandsProvider } from "./providers/commands";

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
	// TODO: Create drag & drop provider here
	const treeProvider = new RojoTreeProvider(apiDump, apiReflection);
	const treeView = vscode.window.createTreeView("rojoExplorer", {
		treeDataProvider: treeProvider,
	});
	context.subscriptions.push(treeView);

	// Create other providers for things such as settings, selection handling, ...
	const settings = new SettingsProvider();
	const selection = new SelectionProvider(treeView, treeProvider);
	const commands = new CommandsProvider(treeView, treeProvider);
	context.subscriptions.push(settings);
	context.subscriptions.push(selection);
	context.subscriptions.push(commands);

	// Listen for settings changing, if any of the settings that
	// change behavior of the sourcemap or the sourcemap watch
	// command change we have to re-initialize the workspace
	settings.listen("includeNonScripts", () => {
		connectAllWorkspaces(settings, treeProvider);
	});
	settings.listen("rojoProjectFile", () => {
		connectAllWorkspaces(settings, treeProvider);
	});

	// Listen for focus changing to sync selection with our tree view items
	context.subscriptions.push(
		vscode.window.onDidChangeActiveTextEditor((e) => {
			if (e && treeView.visible) {
				const filePath = e.document.uri.fsPath;
				const fileItem = treeProvider.find(filePath);
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
				connectWorkspace(addedFolder, settings, treeProvider);
			}
			for (const removedFolder of event.removed) {
				disconnectWorkspace(removedFolder);
			}
		})
	);
	connectAllWorkspaces(settings, treeProvider);
}

export function deactivate() {
	disconnectAllWorkspaces();
}
