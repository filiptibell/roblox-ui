import * as vscode from "vscode"

import { RpcServer } from "./server"
import { Providers } from "./providers"

const workspaceServers: Map<string, RpcServer> = new Map()

let currentProviders: Providers

export const connectAllWorkspaces = async (providers: Providers) => {
	await disconnectAllWorkspaces()

	currentProviders = providers

	const promises = new Array<Promise<void>>()

	if (vscode.workspace.workspaceFolders) {
		for (const workspaceFolder of vscode.workspace.workspaceFolders) {
			const workspacePath = workspaceFolder.uri.fsPath
			const workspaceServer = new RpcServer(providers, workspacePath)
			workspaceServers.set(workspacePath, workspaceServer)
			providers.explorerTree.connectServer(workspacePath, workspaceServer)
		}
	}

	await Promise.all(promises)
}

export const disconnectAllWorkspaces = async () => {
	if (currentProviders) {
		currentProviders.explorerTree.disconnectAllServers()
	}

	const promises = new Array<Promise<void>>()

	for (const [_, server] of workspaceServers) {
		promises.push(server.stop())
	}
	workspaceServers.clear()

	await Promise.all(promises)
}

export const reconnectAllWorkspaces = async () => {
	if (currentProviders) {
		await connectAllWorkspaces(currentProviders)
	}
}
