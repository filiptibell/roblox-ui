import * as vscode from "vscode";
import * as cp from "child_process";
import * as os from "os";
import * as fs from "fs";

import { SettingsProvider } from "../providers/settings";

let outputChannel: vscode.OutputChannel;

export type RpcMessageKind = "Request" | "Response" | "Notification";
export type RpcMessageData = {
	id: number;
	method: string;
	value?: any;
};
export type RpcMessage = {
	kind: RpcMessageKind;
	data: RpcMessageData;
};

const validateRpcMessage = (
	messageString: string
): { valid: true; message: RpcMessage } | { valid: false; err: string } => {
	const message = JSON.parse(messageString);
	if (typeof message !== "object") {
		return {
			valid: false,
			err: `message must be an object, got ${typeof message}`,
		};
	}
	if (typeof message.kind !== "string") {
		return {
			valid: false,
			err: `message.kind must be a string, got ${typeof message.kind}`,
		};
	}
	if (typeof message.data !== "object") {
		return {
			valid: false,
			err: `message.data must be a object, got ${typeof message.data}`,
		};
	}
	if (typeof message.data.id !== "number") {
		return {
			valid: false,
			err: `message.data.id must be a number, got ${typeof message.data
				.id}`,
		};
	}
	if (typeof message.data.method !== "string") {
		return {
			valid: false,
			err: `message.data.method must be a string, got ${typeof message
				.data.method}`,
		};
	}
	return {
		valid: true,
		message,
	};
};

const fileExistsSync = (path: vscode.Uri): boolean => {
	try {
		return fs.existsSync(path.fsPath);
	} catch {
		return false;
	}
};

const findServerExecutable = (context: vscode.ExtensionContext): string => {
	const exeName = os.platform() === "win32" ? "roblox-ui.exe" : "roblox-ui";

	const exeDebug = vscode.Uri.joinPath(
		context.extensionUri,
		"out",
		"debug",
		exeName
	);

	const exeRelease = vscode.Uri.joinPath(
		context.extensionUri,
		"out",
		"release",
		exeName
	);

	const command = fileExistsSync(exeRelease)
		? exeRelease.fsPath
		: fileExistsSync(exeDebug)
		? exeDebug.fsPath
		: null;
	if (!command) {
		throw new Error("Missing server executable");
	}

	return command;
};

export const startServer = (
	context: vscode.ExtensionContext,
	workspacePath: string,
	settings: SettingsProvider,
	callback: (
		child: cp.ChildProcessWithoutNullStreams,
		message: RpcMessage
	) => void
): cp.ChildProcessWithoutNullStreams => {
	if (outputChannel === undefined) {
		outputChannel = vscode.window.createOutputChannel("Roblox UI");
	}

	const settingsJson = JSON.stringify({
		autogenerate: settings.get("sourcemap.autogenerate"),
		rojoProjectFile: settings.get("sourcemap.rojoProjectFile"),
		includeNonScripts: settings.get("sourcemap.includeNonScripts"),
		ignoreGlobs: settings.get("sourcemap.ignoreGlobs"),
	});

	const command = findServerExecutable(context);
	const commandArgs = ["serve"];
	const commandEnv = {
		// eslint-disable-next-line @typescript-eslint/naming-convention
		SETTINGS: settingsJson,
	};

	const childProcess = cp.spawn(command, commandArgs, {
		cwd: workspacePath,
		env: { ...process.env, ...commandEnv },
		shell: true,
	});

	let stdout = "";
	childProcess.stdout.on("data", (data: Buffer) => {
		stdout += data.toString("utf8");
		if (stdout.endsWith("\n")) {
			const result = validateRpcMessage(stdout);
			stdout = "";
			if (result.valid) {
				callback(childProcess, result.message);
			} else {
				outputChannel.appendLine(
					"Failed to parse rpc message:\n" + result.err
				);
			}
		}
	});

	childProcess.stderr.on("data", (data: Buffer) => {
		outputChannel.append(data.toString("utf8"));
	});

	return childProcess;
};
