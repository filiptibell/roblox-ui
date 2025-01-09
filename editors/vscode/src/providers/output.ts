import * as vscode from "vscode"

import { Providers } from "."
import { RpcServer } from "../server"

export class OutputProvider implements vscode.Disposable {
	private readonly channel: vscode.LogOutputChannel
	private readonly disposables: Array<vscode.Disposable> = new Array()

	private server: RpcServer | undefined = undefined
	private disconnect: undefined | (() => boolean) = undefined

	constructor(public readonly providers: Providers) {
		const channel = vscode.window.createOutputChannel("Roblox UI - Output Sync", { log: true })
		this.disposables.push(channel)
		this.channel = channel
	}

	dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose()
		}
	}

	public disconnectServer() {
		if (this.server !== undefined) {
			this.server.stop()
			this.server = undefined
		}
		if (this.disconnect !== undefined) {
			this.disconnect()
			this.disconnect = undefined
		}
	}

	public connectServer(server: RpcServer) {
		const disconnect = server.onRequest("output/notification", (notif) => {
			if (notif !== null) {
				// TODO: Emit trace lines
				if (notif.kind === "Error") {
					this.channel.error(notif.message)
				} else if (notif.kind === "Warning") {
					this.channel.warn(notif.message)
				} else if (notif.kind === "Info") {
					this.channel.info(notif.message)
				} else if (notif.kind === "Debug") {
					this.channel.debug(notif.message)
				}
			}
		})
		this.disconnect = disconnect
		this.server = server
	}
}
