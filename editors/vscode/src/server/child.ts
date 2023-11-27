import * as vscode from "vscode";
import * as cp from "child_process";
import * as os from "os";
import * as fs from "fs";

const treekill = require("tree-kill");
const readline = require("linebyline");

import { SettingsProvider } from "../providers/settings";
import { RpcMessage, isRpcMessage } from "./message";

let outputChannel: vscode.OutputChannel;

const KILL_SIGNALS = ["SIGHUP", "SIGINT", "SIGKILL", "SIGTERM"];

const fileExistsSync = (path: vscode.Uri): boolean => {
	try {
		return fs.existsSync(path.fsPath);
	} catch {
		return false;
	}
};

const findServerExecutable = (context: vscode.ExtensionContext): string => {
	const exeName = os.platform() === "win32" ? "roblox-ui.exe" : "roblox-ui";

	const exeDebug = vscode.Uri.joinPath(context.extensionUri, "out", "debug", exeName);

	const exeRelease = vscode.Uri.joinPath(context.extensionUri, "out", "release", exeName);

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

export const log = (message: string) => {
	if (outputChannel === undefined) {
		outputChannel = vscode.window.createOutputChannel("Roblox UI");
	}
	outputChannel.append(message);
};

export const start = (
	context: vscode.ExtensionContext,
	workspacePath: string,
	settings: SettingsProvider,
	callback: (message: RpcMessage) => void,
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

	readline(childProcess.stdout, {
		maxLineLength: 1024 * 256, // 256 KiB should be enough for any message
		retainBuffer: false,
	}).on("line", (stdout: string) => {
		const message = JSON.parse(stdout);
		if (isRpcMessage(message)) {
			callback(message);
		} else {
			outputChannel.appendLine(`Failed to parse rpc message:\n${stdout}`);
		}
	});

	childProcess.stderr.on("data", (data: Buffer) => {
		outputChannel.append(data.toString("utf8"));
	});

	return childProcess;
};

export const kill = (childProcess: cp.ChildProcessWithoutNullStreams): Promise<void> => {
	return new Promise((resolve, reject) => {
		if (childProcess.pid === undefined) {
			reject("Failed to superkill process: no pid");
			return;
		}

		if (KILL_SIGNALS.length <= 0) {
			reject("Failed to superkill process: no signals");
			return;
		}

		let killErrors = 0;
		let killSuccess = false;
		let killErrorLines = "";

		for (const signal of KILL_SIGNALS) {
			treekill(childProcess.pid, signal, (err: Error | undefined) => {
				if (err) {
					killErrors += 1;
					killErrorLines += "- ";
					killErrorLines += err.toString();
					killErrorLines += "\n";
					if (killErrors === KILL_SIGNALS.length) {
						reject(new Error(`Failed to superkill process:\n${killErrorLines}`));
					}
				} else {
					if (killSuccess !== true) {
						killSuccess = true;
						resolve();
					}
				}
			});
		}
	});
};
