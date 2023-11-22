import * as vscode from "vscode";
import * as cp from "child_process";

import { SettingsProvider } from "../providers/settings";
import { kill, log, start } from "./child";
import { RpcMessage, createRpcRequest, respondToRpcMessage } from "./message";

export * from "./dom";
export * from "./message";

type RpcCallback = (response: RpcMessage) => void;
export type RpcHandler = (data?: any) => any | undefined;

export class RpcServer {
	private child: cp.ChildProcessWithoutNullStreams;
	private handlers: Map<string, RpcHandler> = new Map();
	private resolvers: Map<number, RpcCallback> = new Map();
	private idCounter: number = 0;

	constructor(
		private readonly context: vscode.ExtensionContext,
		private readonly workspacePath: string,
		private readonly settingsProvider: SettingsProvider
	) {
		this.child = start(
			this.context,
			this.workspacePath,
			this.settingsProvider,
			(message) => {
				this.onMessage(message);
			}
		);
	}

	public async stop() {
		await kill(this.child);
		this.idCounter = 0;
	}

	public async restart() {
		await kill(this.child);
		this.idCounter = 0;
		this.child = start(
			this.context,
			this.workspacePath,
			this.settingsProvider,
			(message) => {
				this.onMessage(message);
			}
		);
	}

	public async sendRequest(
		method: string,
		requestValue?: any
	): Promise<any | undefined> {
		this.idCounter += 1;
		const id = this.idCounter;
		return new Promise((resolve) => {
			this.resolvers.set(id, resolve);
			const requestRpc =
				requestValue !== undefined
					? createRpcRequest(method, id, requestValue)
					: createRpcRequest(method, id, null);
			const requestJson = JSON.stringify(requestRpc);
			this.child.stdin.write(requestJson);
			this.child.stdin.write("\n");
		});
	}

	public onRequest(method: string, handler: RpcHandler) {
		this.handlers.set(method, handler);
	}

	private onMessage(message: RpcMessage) {
		if (message.kind === "Request") {
			let handler = this.handlers.get(message.data.method);
			if (handler !== undefined) {
				const responseValue = handler(message.data.value);
				const responseRpc =
					responseValue !== undefined
						? respondToRpcMessage(message, responseValue)
						: respondToRpcMessage(message, null);
				const responseJson = JSON.stringify(responseRpc);
				this.child.stdin.write(responseJson);
				this.child.stdin.write("\n");
			} else {
				log(
					"Missing handler for request!" +
						`\nMethod: "${message.data.method}"` +
						`\nId: ${message.data.id}` +
						`\nValue: ${JSON.stringify(message.data.value)}\n`
				);
			}
		} else if (message.kind === "Response") {
			let resolver = this.resolvers.get(message.data.id);
			if (resolver !== undefined) {
				this.resolvers.delete(message.data.id);
				resolver(message.data.value);
			} else {
				log(
					"Missing resolver for request!" +
						`\nMethod: "${message.data.method}"` +
						`\nId: ${message.data.id}` +
						`\nValue: ${JSON.stringify(message.data.value)}\n`
				);
			}
		}
	}
}
