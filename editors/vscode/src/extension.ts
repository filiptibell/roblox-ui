import * as vscode from "vscode"

import { connectAllWorkspaces, disconnectAllWorkspaces, reconnectAllWorkspaces } from "./workspaces"

import { Providers } from "./providers"
import { SettingsName } from "./providers/settings"

export async function activate(context: vscode.ExtensionContext) {
	let activated = false

	// Create all of the providers
	const providers = new Providers(context)
	context.subscriptions.push(providers)

	// Listen for settings changing, if any of the settings that
	// change behavior of the sourcemap or the sourcemap watch
	// command change we have to re-connect the workspaces
	const settingNames: SettingsName[] = [
		"sourcemap.autogenerate",
		"sourcemap.ignoreGlobs",
		"sourcemap.includeNonScripts",
		"sourcemap.rojoProjectFile",
		"explorer.showClassNames",
		"explorer.showFilePaths",
		"explorer.iconPack",
		"explorer.customIconDir",
		"wally.modifyPackagesDir",
		"wally.showPackageVersion",
	]
	for (const settingName of settingNames) {
		context.subscriptions.push(
			providers.settings.listen(settingName, () => {
				if (activated) {
					reconnectAllWorkspaces()
				}
			})
		)
	}

	// When custom icons get recompiled, we also need to re-connect
	// the workspaces, since the icon file paths are now different
	context.subscriptions.push(
		providers.icons.onDidChangeCustomIcons(() => {
			reconnectAllWorkspaces()
		})
	)

	// Listen for workspace folders changing, and re-connect
	// current workspace folders when that happens as well
	context.subscriptions.push(vscode.workspace.onDidChangeWorkspaceFolders(reconnectAllWorkspaces))

	// Finally, connect workspaces once initially
	connectAllWorkspaces(providers)

	activated = true
}

export async function deactivate() {
	await disconnectAllWorkspaces()
}
