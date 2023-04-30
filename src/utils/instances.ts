import * as vscode from "vscode";
import * as fs from "fs/promises";
import * as path from "path";

import * as fsSync from "fs";

import { extractRojoFileExtension, isInitFilePath } from "./rojo";

export type CreationResult = {
	name: string;
	className: string;
	folderPath: string | undefined;
	filePaths: string[] | undefined;
};

export type RenameResult = {
	name: string | undefined;
	folderPath: string | undefined;
	filePaths: string[] | undefined;
};

const getInstanceFileName = (
	className: string,
	instanceName: string
): string => {
	if (className === "Script") {
		return `${instanceName}.server.luau`;
	} else if (className === "LocalScript") {
		return `${instanceName}.client.luau`;
	} else {
		return `${instanceName}.luau`;
	}
};

const pathExists = async (path: string) => {
	return new Promise((resolve, _) => {
		fsSync.access(path, fsSync.constants.F_OK, (err) => {
			if (err) {
				resolve(false);
			} else {
				resolve(true);
			}
		});
	});
};

const canCreateInstanceFile = async (
	folderPath: string | null,
	filePath: string | null,
	className: string,
	instanceName: string
): Promise<string | undefined> => {
	if (!folderPath) {
		if (!filePath) {
			return "Missing file path (internal error)";
		}
		if (isInitFilePath(filePath)) {
			folderPath = path.dirname(filePath);
		} else {
			const fileExt = extractRojoFileExtension(filePath);
			if (fileExt) {
				const dirName = path.dirname(filePath);
				const subdirName = path.basename(filePath, `.${fileExt}`);
				folderPath = path.join(dirName, subdirName);
			} else {
				return "Invalid file extension (internal error)";
			}
		}
	}

	if (className === "Folder") {
		const newFolderPath = path.join(folderPath, instanceName);
		if (!(await pathExists(newFolderPath))) {
			return;
		}
		if ((await fs.stat(newFolderPath)).isDirectory()) {
			return `Folder already exists at '${newFolderPath}'`;
		}
	} else {
		const newFileName = getInstanceFileName(className, instanceName);
		const newFilePath = path.join(folderPath, newFileName);
		if (!(await pathExists(newFilePath))) {
			return;
		}
		if ((await fs.stat(newFilePath)).isFile()) {
			return `File already exists at '${newFileName}'`;
		}
	}

	return;
};

export const createNewInstance = async (
	folderPath: string | null,
	filePath: string | null,
	className: string,
	instanceName: string
): Promise<[boolean, CreationResult | undefined]> => {
	// Make sure we got a folder path
	if (!folderPath && !filePath) {
		vscode.window.showWarningMessage(
			`Failed to insert new ${className} instance!` +
				"\n\nThe selected instance had no folder or file path."
		);
		return [false, undefined];
	}

	await forceCloseMatchingTextDocuments(folderPath, filePath);

	// If we got a file path that is not an init file, then we
	// need to convert the file into a subfolder + init file
	let isInitFile = false;
	if (!folderPath) {
		if (!filePath) {
			throw new Error("Unreachable");
		}
		if (isInitFilePath(filePath)) {
			folderPath = path.dirname(filePath);
		} else {
			const fileExt = extractRojoFileExtension(filePath);
			if (fileExt) {
				const dirName = path.dirname(filePath);
				const subdirName = path.basename(filePath, `.${fileExt}`);
				folderPath = path.join(dirName, subdirName);
				await fs.mkdir(folderPath);
				await fs.rename(filePath, `${folderPath}/init.${fileExt}`);
				isInitFile = true;
			} else {
				vscode.window.showWarningMessage(
					`Failed to insert new ${className} instance!` +
						"\n\nThe selected instance had an invalid file extension."
				);
				return [false, undefined];
			}
		}
	}

	// Create the folder or instance file
	if (className === "Folder") {
		const newFolderPath = path.join(folderPath, instanceName);
		await fs.mkdir(newFolderPath);
		return [
			true,
			{
				name: instanceName,
				className,
				folderPath,
				filePaths: undefined,
			},
		];
	} else {
		const newFileName = getInstanceFileName(className, instanceName);
		const newFilePath = path.join(folderPath, newFileName);
		await fs.writeFile(newFilePath, "");
		return [
			true,
			{
				name: instanceName,
				className,
				folderPath: isInitFile ? folderPath : undefined,
				filePaths: [newFilePath],
			},
		];
	}
};

export const deleteExistingInstance = async (
	folderPath: string | null,
	filePath: string | null
): Promise<boolean> => {
	// Make sure we got a folder path
	if (!folderPath && !filePath) {
		vscode.window.showWarningMessage(
			`Failed to delete instance!` +
				"\n\nThe selected instance had no folder or file path."
		);
		return false;
	}

	await forceCloseMatchingTextDocuments(folderPath, filePath);

	// Init files should have both their folder & file paths deleted
	if (folderPath && filePath && isInitFilePath(filePath)) {
		await fs.rm(folderPath, {
			recursive: true,
		});
		return true;
	}

	// Folders should also be deleted recursively
	if (folderPath && !filePath) {
		await fs.rm(folderPath, {
			recursive: true,
		});
		return true;
	}

	// Files should just be deleted as they are
	// TODO: Delete meta files along with the normal instance file
	// TODO: If the folder the instance is in was using an init file and no longer has
	// any children, try to remove the usage of init files and make it a plain file instead
	if (filePath) {
		await fs.rm(filePath);
		return true;
	}

	vscode.window.showWarningMessage(
		`Failed to delete instance!` +
			"\n\nThe selected instance had an unknown path kind."
	);
	return false;
};

export const renameExistingInstance = async (
	folderPath: string | null,
	filePath: string | null,
	instanceName: string
): Promise<[boolean, RenameResult | undefined]> => {
	// Make sure we got a folder path
	if (!folderPath && !filePath) {
		vscode.window.showWarningMessage(
			`Failed to rename instance!` +
				"\n\nThe selected instance had no folder or file path."
		);
		return [false, undefined];
	}

	// Init files should have their parent folder renamed
	if (folderPath && filePath && isInitFilePath(filePath)) {
		const newFolderPath = path.join(folderPath, "..", instanceName);
		await fs.rename(folderPath, newFolderPath);
		return [
			true,
			{
				name: instanceName,
				folderPath: newFolderPath,
				filePaths: undefined,
			},
		];
	}

	// Folders should also be renamed the same way
	if (folderPath && !filePath) {
		const newFolderPath = path.join(folderPath, "..", instanceName);
		await fs.rename(folderPath, newFolderPath);
		return [
			true,
			{
				name: instanceName,
				folderPath: newFolderPath,
				filePaths: undefined,
			},
		];
	}

	// Files should just be renamed as they are
	// TODO: Rename any corresponding meta files, too
	if (filePath) {
		const fileExt = extractRojoFileExtension(filePath);
		if (fileExt) {
			const fileDir = path.dirname(filePath);
			const fileName = `${instanceName}.${fileExt}`;
			await fs.rename(filePath, path.join(fileDir, fileName));
			return [
				true,
				{
					name: instanceName,
					folderPath: undefined,
					filePaths: [filePath],
				},
			];
		} else {
			vscode.window.showWarningMessage(
				`Failed to rename instance!` +
					"\n\nThe selected instance had an unknown path file extension."
			);
			return [false, undefined];
		}
	}

	vscode.window.showWarningMessage(
		`Failed to rename instance!` +
			"\n\nThe selected instance had an unknown path kind."
	);
	return [false, undefined];
};

export const promptNewInstanceCreation = async (
	folderPath: string | null,
	filePath: string | null,
	classNameOrInsertService: string | boolean | void
): Promise<[boolean, CreationResult | undefined]> => {
	// TODO: Better classes to pick from
	if (classNameOrInsertService === true) {
		vscode.window.showInformationMessage("TODO");
		return [false, undefined];
	}
	const items = [
		new InstanceInsertItem("Folder"),
		new InstanceInsertItem("ModuleScript"),
		new InstanceInsertItem("LocalScript"),
		new InstanceInsertItem("Script"),
	];
	const className =
		typeof classNameOrInsertService === "string"
			? classNameOrInsertService
			: await vscode.window.showQuickPick(items);
	if (className) {
		const chosen =
			typeof className !== "string" ? className.className : className;
		const instanceName = await vscode.window.showInputBox({
			prompt: "Type in a name for the new instance",
			value: chosen,
			validateInput: async (value: string) => {
				try {
					return await canCreateInstanceFile(
						folderPath,
						filePath,
						chosen,
						value
					);
				} catch (e) {
					return `Internal error: ${e}`;
				}
			},
		});
		if (instanceName) {
			try {
				return await createNewInstance(
					folderPath,
					filePath,
					chosen,
					instanceName
				);
			} catch (e) {
				vscode.window.showWarningMessage(
					`Failed to insert new instance!` +
						`\n\nError message:\n\n${e}`
				);
			}
		}
	}
	return [false, undefined];
};

export const promptRenameExistingInstance = async (
	folderPath: string | null,
	filePath: string | null
): Promise<[boolean, RenameResult | undefined]> => {
	return new Promise((resolve, reject) => {
		vscode.window
			.showInputBox({
				prompt: "Type in a new name for the instance",
				value: "",
			})
			.then(async (instanceName) => {
				if (instanceName) {
					try {
						resolve(
							await renameExistingInstance(
								folderPath,
								filePath,
								instanceName
							)
						);
					} catch (e) {
						vscode.window.showWarningMessage(
							`Failed to rename instance!` +
								`\n\nError message:\n\n${e}`
						);
						reject(e);
					}
				} else {
					resolve([false, undefined]);
				}
			});
	});
};

class InstanceInsertItem implements vscode.QuickPickItem {
	label: string;
	description?: string;

	constructor(public readonly className: string) {
		this.label = className;
		// TODO: Descriptions?
		if (className === "Folder") {
		} else if (className === "ModuleScript") {
		} else if (className === "LocalScript") {
		} else if (className === "Script") {
		}
	}
}

const forceCloseMatchingTextDocuments = async (
	folderPath: string | null,
	filePath: string | null
): Promise<[boolean, boolean]> => {
	let wasClosed = false;
	let hadFocus = false;
	for (const doc of vscode.workspace.textDocuments) {
		if (
			(filePath && doc.uri.fsPath === filePath) ||
			(folderPath && doc.uri.fsPath.startsWith(folderPath))
		) {
			const [res1, res2] = await forceCloseTextDocument(doc.uri);
			wasClosed = wasClosed || res1;
			hadFocus = hadFocus || res2;
		}
	}
	return [wasClosed, hadFocus];
};

const forceCloseTextDocument = async (
	uri: vscode.Uri
): Promise<[boolean, boolean]> => {
	// HACK: To properly close an editor that is opened we
	// have to first force show it, force it to be active,
	// and then close the currently active editor
	let wasClosed = false;
	let hadFocus = false;
	for (const doc of vscode.workspace.textDocuments) {
		if (doc.uri.fsPath === uri.fsPath) {
			const textEditor = await vscode.window.showTextDocument(
				doc,
				undefined,
				false
			);
			wasClosed = true;
			hadFocus =
				hadFocus ||
				textEditor.document.uri.fsPath ===
					vscode.window.activeTextEditor?.document.uri.fsPath;
			try {
				textEditor.hide();
			} catch {
				vscode.commands.executeCommand(
					"workbench.action.closeActiveEditor",
					textEditor.document.uri
				);
			}
		}
	}
	return [wasClosed, hadFocus];
};

const forceShowTextDocument = async (uri: vscode.Uri) => {
	if ((await fs.stat(uri.fsPath)).isFile()) {
		const textDoc = await vscode.workspace.openTextDocument(uri);
		await vscode.window.showTextDocument(textDoc);
	}
};
