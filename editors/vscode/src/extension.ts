import * as vscode from "vscode";

import {
	connectAllWorkspaces,
	connectWorkspace,
	disconnectAllWorkspaces,
	disconnectWorkspace,
	refreshAllWorkspaces,
	reloadAllWorkspaces,
} from "./workspaces";

import { RojoTreeProvider } from "./providers/explorer";
import { SettingsProvider } from "./providers/settings";
import { SelectionProvider } from "./providers/selection";
import { CommandsProvider } from "./providers/commands";

import { MetadataProvider } from "./providers/metadata";
import { IconsProvider } from "./providers/icons";

export async function activate(context: vscode.ExtensionContext) {
	// Create settings provider first, it is used by other providers
	const settings = new SettingsProvider();
	context.subscriptions.push(settings);

	// Create metadata and icon providers, these will load some bundled files
	// using sync fs methods so we do it before creating anything else below
	const metadata = new MetadataProvider(context);
	const icons = new IconsProvider(context);
	context.subscriptions.push(metadata);
	context.subscriptions.push(icons);

	// Create the main tree view and data providers
	// TODO: Create drag & drop provider here
	const treeProvider = new RojoTreeProvider(settings, metadata, icons);
	const treeView = vscode.window.createTreeView("roblox-ui.explorer", {
		treeDataProvider: treeProvider,
		showCollapseAll: true,
		canSelectMany: false,
	});
	context.subscriptions.push(treeView);

	// Create other providers for things such as selection handling, ...
	const commands = new CommandsProvider(
		context,
		metadata,
		treeView,
		treeProvider
	);
	const selection = new SelectionProvider(treeView, treeProvider);
	context.subscriptions.push(commands);
	context.subscriptions.push(selection);

	// Listen for settings changing, if any of the settings that
	// change behavior of the sourcemap or the sourcemap watch
	// command change we have to re-initialize the workspace
	settings.listen("sourcemap.autogenerate", reloadAllWorkspaces);
	settings.listen("sourcemap.ignoreGlobs", reloadAllWorkspaces);
	settings.listen("sourcemap.includeNonScripts", reloadAllWorkspaces);
	settings.listen("sourcemap.rojoProjectFile", reloadAllWorkspaces);
	settings.listen("wally.modifyPackagesDir", reloadAllWorkspaces);

	// For some settings we don't need a complete reload of the project,
	// sourcemap, and tree, we just need a refresh of any existing tree
	settings.listen("explorer.showClassNames", refreshAllWorkspaces);
	settings.listen("explorer.showFilePaths", refreshAllWorkspaces);
	settings.listen("explorer.iconPack", refreshAllWorkspaces);
	settings.listen("wally.showPackageVersion", refreshAllWorkspaces);

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
