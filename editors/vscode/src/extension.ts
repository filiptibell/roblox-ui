import * as vscode from "vscode";

import {
	connectAllWorkspaces,
	disconnectAllWorkspaces,
	reconnectAllWorkspaces,
} from "./workspaces";

import { SettingsProvider } from "./providers/settings";
import { SelectionProvider } from "./providers/selection";
import { CommandsProvider } from "./providers/commands";

import { MetadataProvider } from "./providers/metadata";
import { IconsProvider } from "./providers/icons";
import { ExplorerTreeProvider } from "./explorer";
import { QuickOpenProvider } from "./providers/quickOpen";

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
	const explorerTree = new ExplorerTreeProvider(settings, metadata, icons);
	const explorerView = vscode.window.createTreeView("roblox-ui.explorer", {
		treeDataProvider: explorerTree,
		showCollapseAll: true,
		canSelectMany: false,
	});
	context.subscriptions.push(explorerView);

	// Create other providers for things such as selection handling, ...
	const quickOpen = new QuickOpenProvider(
		settings,
		metadata,
		icons,
		explorerTree
	);
	const commands = new CommandsProvider(
		context,
		metadata,
		explorerView,
		explorerTree,
		quickOpen
	);
	const selection = new SelectionProvider(explorerTree);
	context.subscriptions.push(quickOpen);
	context.subscriptions.push(commands);
	context.subscriptions.push(selection);

	// Listen for settings changing, if any of the settings that
	// change behavior of the sourcemap or the sourcemap watch
	// command change we have to re-connect the workspaces
	settings.listen("sourcemap.autogenerate", reconnectAllWorkspaces);
	settings.listen("sourcemap.ignoreGlobs", reconnectAllWorkspaces);
	settings.listen("sourcemap.includeNonScripts", reconnectAllWorkspaces);
	settings.listen("sourcemap.rojoProjectFile", reconnectAllWorkspaces);
	settings.listen("explorer.showClassNames", reconnectAllWorkspaces);
	settings.listen("explorer.showFilePaths", reconnectAllWorkspaces);
	settings.listen("explorer.iconPack", reconnectAllWorkspaces);
	settings.listen("wally.modifyPackagesDir", reconnectAllWorkspaces);
	settings.listen("wally.showPackageVersion", reconnectAllWorkspaces);

	// Listen for workspace folders changing, and re-connect
	// current workspace folders when that happens as well
	context.subscriptions.push(
		vscode.workspace.onDidChangeWorkspaceFolders(reconnectAllWorkspaces)
	);
	connectAllWorkspaces(context, settings, explorerTree);
}

export async function deactivate() {
	await disconnectAllWorkspaces();
}
