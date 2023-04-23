import * as vscode from "vscode";
import * as fs from "fs/promises";
import * as cp from "child_process";

import { RojoTreeProvider } from "./provider";
import { SettingsManager } from "./utils/settings";
import {
	parseSourcemap,
	rojoSourcemapWatchIsSupported,
} from "./utils/sourcemap";

const workspaceDestructors: Map<string, Function> = new Map();
const workspaceUpdaters: Map<string, Function> = new Map();

export const updateWorkspace = (folder: vscode.WorkspaceFolder) => {
	const update = workspaceUpdaters.get(folder.uri.fsPath);
	if (update) {
		update();
	}
};

export const updateAllWorkspaces = () => {
	const workspaceFolders = vscode.workspace.workspaceFolders;
	if (workspaceFolders) {
		workspaceFolders.forEach(updateWorkspace);
	}
};

export const connectWorkspace = (
	folder: vscode.WorkspaceFolder,
	settings: SettingsManager,
	treeDataProvider: RojoTreeProvider
) => {
	const workspacePath = folder.uri.fsPath;

	// Check for autogeneration setting, and ensure sourcemap
	// with watching is supported if we want to autogenerate
	let autogenerate = settings.get("autogenerateSourcemap");
	if (autogenerate) {
		if (!rojoSourcemapWatchIsSupported(workspacePath)) {
			autogenerate = false;
		}
	}

	if (autogenerate) {
		// Spawn a new rojo process that will generate sourcemaps and watch for changes
		const childArgs = [
			"sourcemap",
			"--watch",
			settings.get("includeNonScripts") ? "--include-non-scripts" : "",
			settings.get("rojoProjectFile") || "default.project.json",
		];
		const childProcess = cp.spawn("rojo", childArgs, {
			cwd: workspacePath,
			env: process.env,
			shell: true,
		});

		// Listen for new sourcemaps being generated and output, here we will have to
		// keep track of stdout since data may be received in pieces and incomplete json
		// When we have complete parseable json we will update + clear the current stdout
		let stdout = "";
		childProcess.stdout.on("data", (data: Buffer) => {
			stdout += data.toString("utf8");
			try {
				let sourcemap = parseSourcemap(stdout);
				treeDataProvider.update(workspacePath, sourcemap);
				stdout = "";
			} catch {}
		});

		// Listen for error messagess and the child process closing
		let stderr = "";
		let killed = false;
		childProcess.stderr.on("data", (data: Buffer) => {
			stderr += data.toString("utf8");
		});
		childProcess.on("close", (code: number) => {
			if (killed) {
				return;
			}
			if (code !== 0) {
				if (stderr.length > 0) {
					vscode.window.showErrorMessage(
						"Rojo Explorer failed to generate a sourcemap!" +
							`\nRojo exited with code ${code}` +
							`\nMessage:\n${stderr}`
					);
				} else {
					vscode.window.showErrorMessage(
						"Rojo Explorer failed to generate a sourcemap!" +
							`\nRojo exited with code ${code}`
					);
				}
			}
		});

		// Create callback for disconnecting (destroying)
		// everything created for this workspace folder
		const destroy = () => {
			workspaceUpdaters.delete(workspacePath);
			workspaceDestructors.delete(workspacePath);
			treeDataProvider.delete(workspacePath);
			killed = true;
			childProcess.kill();
		};

		// Store callbacks to access them from other listeners
		workspaceUpdaters.set(workspacePath, () => {});
		workspaceDestructors.set(workspacePath, destroy);

		return;
	}

	// Autogeneration is either disabled or not available, so we will
	// instead watch the sourcemap.json file in this workspace folder
	const sourcemapPath = `${workspacePath}/sourcemap.json`;
	const fileWatcher = vscode.workspace.createFileSystemWatcher(sourcemapPath);

	// Create callback for updating sourcemap
	const update = async () => {
		const sourcemapJson = await fs.readFile(sourcemapPath, "utf8");
		treeDataProvider.update(workspacePath, parseSourcemap(sourcemapJson));
	};

	// Create callback for disconnecting (destroying)
	// everything created for this workspace folder
	const destroy = () => {
		workspaceUpdaters.delete(workspacePath);
		workspaceDestructors.delete(workspacePath);
		treeDataProvider.delete(workspacePath);
		fileWatcher.dispose();
	};

	// Store callbacks to access them from other listeners
	workspaceUpdaters.set(workspacePath, update);
	workspaceDestructors.set(workspacePath, destroy);

	// Start watching the sourcemap for changes and update once initially
	fileWatcher.onDidChange(update);
	update();
};

export const connectAllWorkspaces = (
	settings: SettingsManager,
	provider: RojoTreeProvider
) => {
	if (vscode.workspace.workspaceFolders) {
		vscode.workspace.workspaceFolders.forEach((folder) => {
			disconnectWorkspace(folder);
		});
		vscode.workspace.workspaceFolders.forEach((folder) => {
			connectWorkspace(folder, settings, provider);
		});
	}
};

export const disconnectWorkspace = (folder: vscode.WorkspaceFolder) => {
	const destroy = workspaceDestructors.get(folder.uri.fsPath);
	if (destroy) {
		destroy();
	}
};

export const disconnectAllWorkspaces = () => {
	let workspacePaths = [...workspaceDestructors.keys()];
	workspacePaths.forEach((workspacePath) => {
		const destroy = workspaceDestructors.get(workspacePath);
		if (destroy) {
			destroy();
		}
	});
};
