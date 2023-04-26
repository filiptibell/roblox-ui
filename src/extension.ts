import * as vscode from "vscode";

import {
	connectAllWorkspaces,
	connectWorkspace,
	disconnectAllWorkspaces,
	disconnectWorkspace,
} from "./workspaces";

import { RojoTreeProvider } from "./providers/explorer";
import { SettingsProvider } from "./providers/settings";
import { SelectionProvider } from "./providers/selection";
import { CommandsProvider } from "./providers/commands";

import { initRobloxCache } from "./web/roblox";

export async function activate(context: vscode.ExtensionContext) {
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

	// Create settings provider first, since it is used by other providers
	const settings = new SettingsProvider();
	context.subscriptions.push(settings);

	// Create the main tree view and data providers
	// TODO: Create drag & drop provider here
	const treeProvider = new RojoTreeProvider(
		settings,
		cache.cachedApiDump,
		cache.cachedReflection
	);
	const treeView = vscode.window.createTreeView("rojoViewer.explorer", {
		treeDataProvider: treeProvider,
	});
	context.subscriptions.push(treeView);

	// Create other providers for things such as selection handling, ...
	const commands = new CommandsProvider(context, treeView, treeProvider);
	const selection = new SelectionProvider(treeView, treeProvider);
	context.subscriptions.push(commands);
	context.subscriptions.push(selection);

	const forceRefreshAll = () => {
		connectAllWorkspaces(settings, treeProvider);
	};

	// Listen for settings changing, if any of the settings that
	// change behavior of the sourcemap or the sourcemap watch
	// command change we have to re-initialize the workspace
	settings.listen("ignoreGlobs", forceRefreshAll);
	settings.listen("includeNonScripts", forceRefreshAll);
	settings.listen("rojoProjectFile", forceRefreshAll);

	// NOTE: We could move the listeners for these settings into the tree items
	// themselves, but just reloading the workspaces is much much more efficient
	settings.listen("showClassNames", forceRefreshAll);
	settings.listen("showFilePaths", forceRefreshAll);

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
	forceRefreshAll();
}

export function deactivate() {
	disconnectAllWorkspaces();
}
