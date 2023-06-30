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

import { initRobloxCache } from "./web/roblox";
import { IconsProvider } from "./providers/icons";

export async function activate(context: vscode.ExtensionContext) {
	// Create settings provider first, it is used by other providers
	const settings = new SettingsProvider();
	context.subscriptions.push(settings);

	// Fetch api dump and reflection metadata, if the user does not
	// have an internet connection the very first time they activate
	// the extension this may fail but will otherwise fall back to a
	// cached version and warn the user about the potential desync
	const cache = await initRobloxCache(context);
	if (
		!cache.cachedVersion ||
		!cache.cachedApiDump ||
		!cache.cachedReflection
	) {
		return;
	}

	// Create the tree icons provider for instance class icons in the explorer
	const icons = new IconsProvider(
		context,
		cache.cachedApiDump,
		cache.cachedReflection
	);
	context.subscriptions.push(icons);

	// Create the main tree view and data providers
	// TODO: Create drag & drop provider here
	const treeProvider = new RojoTreeProvider(
		settings,
		icons,
		cache.cachedApiDump,
		cache.cachedReflection
	);
	const treeView = vscode.window.createTreeView("rojoViewer.explorer", {
		treeDataProvider: treeProvider,
	});
	context.subscriptions.push(treeView);
	context.subscriptions.push(
		treeProvider.onAutoExpandRootDesired((root) => {
			treeView.reveal(root, {
				expand: true,
				focus: false,
				select: false,
			});
		})
	);

	// Create other providers for things such as selection handling, ...
	const commands = new CommandsProvider(
		context,
		treeView,
		treeProvider,
		icons,
		cache.cachedApiDump,
		cache.cachedReflection
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

	// For some settings we don't need a complete reload of the project,
	// sourcemap, and tree, we just need a refresh of any existing tree
	settings.listen("explorer.showClassNames", refreshAllWorkspaces);
	settings.listen("explorer.showFilePaths", refreshAllWorkspaces);
	settings.listen("explorer.iconPack", refreshAllWorkspaces);

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
