import * as vscode from "vscode"
import * as cp from "child_process"
import * as os from "os"
import * as fs from "fs"

const treekill = require("tree-kill")
const readline = require("linebyline")

import { RpcMessage, isRpcMessage } from "./message"
import { Providers } from "../providers"

const outputChannel = vscode.window.createOutputChannel("Roblox UI")

const KILL_SIGNALS = ["SIGHUP", "SIGINT", "SIGKILL", "SIGTERM"]

const fileExistsSync = (path: vscode.Uri): boolean => {
	try {
		return fs.existsSync(path.fsPath)
	} catch {
		return false
	}
}

const findServerExecutable = (context: vscode.ExtensionContext): string => {
	const exeName = os.platform() === "win32" ? "roblox-ui.exe" : "roblox-ui"

	const exeDebug = vscode.Uri.joinPath(context.extensionUri, "out", "debug", exeName)

	const exeRelease = vscode.Uri.joinPath(context.extensionUri, "out", "release", exeName)

	const command = fileExistsSync(exeRelease)
		? exeRelease.fsPath
		: fileExistsSync(exeDebug)
		? exeDebug.fsPath
		: null
	if (!command) {
		throw new Error("Missing server executable")
	}

	return command
}

export const log = (message: string) => {
	outputChannel.append(message)
}

export const start = (
	providers: Providers,
	workspacePath: string,
	callback: (message: RpcMessage) => void
): cp.ChildProcessWithoutNullStreams => {
	const settingsJson = JSON.stringify({
		autogenerate: providers.settings.get("sourcemap.autogenerate"),
		rojoProjectFile: providers.settings.get("sourcemap.rojoProjectFile"),
		includeNonScripts: providers.settings.get("sourcemap.includeNonScripts"),
		ignoreGlobs: providers.settings.get("sourcemap.ignoreGlobs"),
	})

	const command = findServerExecutable(providers.extensionContext)
	const commandArgs = ["serve"]
	const commandEnv = {
		// eslint-disable-next-line @typescript-eslint/naming-convention
		SETTINGS: settingsJson,
	}

	const childProcess = cp.spawn(command, commandArgs, {
		cwd: workspacePath,
		env: { ...process.env, ...commandEnv },
		shell: true,
	})

	readline(childProcess.stdout, {
		maxLineLength: 1024 * 512, // 512 KiB should be enough for any message
		retainBuffer: true,
	}).on("line", (stdout: string) => {
		try {
			const message = JSON.parse(stdout)
			if (isRpcMessage(message)) {
				callback(message)
			} else {
				outputChannel.appendLine(`Failed to parse rpc message:\n${stdout}`)
			}
		} catch (e) {
			outputChannel.appendLine(`Failed to parse rpc json:\n${e}\nContents:\n${stdout}`)
		}
	})

	childProcess.stderr.on("data", (data: Buffer) => {
		outputChannel.append(data.toString("utf8"))
	})

	return childProcess
}

export const kill = (childProcess: cp.ChildProcessWithoutNullStreams): Promise<void> => {
	return new Promise((resolve, reject) => {
		if (childProcess.pid === undefined) {
			reject("Failed to superkill process: no pid")
			return
		}

		if (KILL_SIGNALS.length <= 0) {
			reject("Failed to superkill process: no signals")
			return
		}

		let killErrors = 0
		let killSuccess = false
		let killErrorLines = ""

		for (const signal of KILL_SIGNALS) {
			treekill(childProcess.pid, signal, (err: Error | undefined) => {
				if (err) {
					killErrors += 1
					killErrorLines += "- "
					killErrorLines += err.toString()
					killErrorLines += "\n"
					if (killErrors === KILL_SIGNALS.length) {
						reject(new Error(`Failed to superkill process:\n${killErrorLines}`))
					}
				} else {
					if (killSuccess !== true) {
						killSuccess = true
						resolve()
					}
				}
			})
		}
	})
}

export const runCommand = async (providers: Providers, args: string[]): Promise<string> => {
	const path = findServerExecutable(providers.extensionContext)
	const childProcess = cp.spawn(path, args)

	return new Promise((resolve, reject) => {
		let stdout = ""
		let stderr = ""
		childProcess.stdout.on("data", (data) => {
			stdout += data
		})
		childProcess.stderr.on("data", (data) => {
			stderr += data
		})
		childProcess.on("close", (code) => {
			if (code === 0) {
				resolve(stdout)
			} else {
				reject(new Error(`Command exited with code ${code}\n${stderr}`))
			}
		})
	})
}
