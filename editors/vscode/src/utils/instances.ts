import * as vscode from "vscode";
import * as path from "path";

const fs = vscode.workspace.fs;

import { extractRojoFileExtension, isInitFilePath } from "./rojo";
import { pathMetadata } from "./files";
import { IconsProvider } from "../providers/icons";
import { SettingsProvider } from "../providers/settings";
import { MetadataProvider } from "../providers/metadata";

const INSERTABLE_SERVICES = new Set([
	"Lighting",
	"LocalizationService",
	"MaterialService",
	"Players",
	"ReplicatedFirst",
	"ReplicatedStorage",
	"ServerScriptService",
	"ServerStorage",
	"SoundService",
	"StarterGui",
	"StarterPack",
	"StarterPlayer",
	"Teams",
	"TextChatService",
	"VoiceChatService",
	"Workspace",
]);

const COMMON_INSTANCES = new Set(["ModuleScript", "LocalScript", "Script"]);

const INSTANCE_JSON_FILE_CONTENTS = `{
    "className": "<<CLASSNAME>>",
    "properties": {

    }
}
`;

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
	} else if (className === "ModuleScript") {
		return `${instanceName}.luau`;
	} else {
		return `${instanceName}.model.json`;
	}
};

const getInstanceFileContents = (
	className: string,
	instanceName: string
): string => {
	if (
		className === "Script" ||
		className === "LocalScript" ||
		className === "ModuleScript"
	) {
		return "";
	} else {
		return INSTANCE_JSON_FILE_CONTENTS.replace("<<CLASSNAME>>", className);
	}
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
		const newMetadata = await pathMetadata(newFolderPath);
		if (!newMetadata.exists) {
			return;
		}
		if (newMetadata.isDir) {
			return `Folder already exists at '${newFolderPath}'`;
		}
	} else {
		const newFileName = getInstanceFileName(className, instanceName);
		const newFilePath = path.join(folderPath, newFileName);
		const newMetadata = await pathMetadata(newFilePath);
		if (!newMetadata.exists) {
			return;
		}
		if (newMetadata.isFile) {
			return `File already exists at '${newFileName}'`;
		}
	}

	return;
};

export const createNewInstance = async (
	folderPath: string | null,
	filePath: string | null,
	className: string,
	instanceName: string,
	isService: boolean | undefined | null | void
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
				// Convert model json files to meta json, since
				// model json does not support init-style files
				const initFileName =
					fileExt === "model.json"
						? "init.meta.json"
						: `init.${fileExt}`;
				folderPath = path.join(dirName, subdirName);
				await fs.createDirectory(vscode.Uri.file(folderPath));
				await fs.rename(
					vscode.Uri.file(filePath),
					vscode.Uri.file(`${folderPath}/${initFileName}`)
				);
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
		await fs.createDirectory(vscode.Uri.file(newFolderPath));
		return [
			true,
			{
				name: instanceName,
				className,
				folderPath,
				filePaths: undefined,
			},
		];
	} else if (isService === true) {
		const newFolderPath = path.join(folderPath, instanceName);
		const newFileName = "init.meta.json";
		const newFilePath = path.join(newFolderPath, newFileName);
		const newFileContents = getInstanceFileContents(
			className,
			instanceName
		);
		await fs.createDirectory(vscode.Uri.file(newFolderPath));
		await fs.writeFile(
			vscode.Uri.file(newFilePath),
			Uint8Array.from(newFileContents, (c) => c.charCodeAt(0))
		);
		return [
			true,
			{
				name: instanceName,
				className,
				folderPath: newFolderPath,
				filePaths: [newFilePath],
			},
		];
	} else {
		const newFileName = getInstanceFileName(className, instanceName);
		const newFilePath = path.join(folderPath, newFileName);
		const newFileContents = getInstanceFileContents(
			className,
			instanceName
		);
		await fs.writeFile(
			vscode.Uri.file(newFilePath),
			Uint8Array.from(newFileContents, (c) => c.charCodeAt(0))
		);
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
		await fs.delete(vscode.Uri.file(folderPath), {
			recursive: true,
		});
		return true;
	}

	// Folders should also be deleted recursively
	if (folderPath && !filePath) {
		await fs.delete(vscode.Uri.file(folderPath), {
			recursive: true,
		});
		return true;
	}

	// Files should just be deleted as they are
	// TODO: Delete meta files along with the normal instance file
	// TODO: If the folder the instance is in was using an init file and no longer has
	// any children, try to remove the usage of init files and make it a plain file instead
	if (filePath) {
		await fs.delete(vscode.Uri.file(filePath));
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
		await fs.rename(
			vscode.Uri.file(folderPath),
			vscode.Uri.file(newFolderPath)
		);
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
		await fs.rename(
			vscode.Uri.file(folderPath),
			vscode.Uri.file(newFolderPath)
		);
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
			await fs.rename(
				vscode.Uri.file(filePath),
				vscode.Uri.file(path.join(fileDir, fileName))
			);
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
	settingsProvider: SettingsProvider,
	metadataProvider: MetadataProvider,
	iconsProvider: IconsProvider,
	folderPath: string | null,
	filePath: string | null,
	classNameOrInsertService: string | boolean | void
): Promise<[boolean, CreationResult | undefined]> => {
	const items: (InstanceInsertItem | InstanceInsertSeparator)[] = [];
	if (typeof classNameOrInsertService !== "string") {
		if (classNameOrInsertService === true) {
			for (const serviceName of INSERTABLE_SERVICES.values()) {
				items.push(new InstanceInsertItem(serviceName));
			}
		} else {
			for (const serviceName of COMMON_INSTANCES.values()) {
				items.push(new InstanceInsertItem(serviceName));
			}
			items.push(new InstanceInsertSeparator());
			const restItems = [];
			for (const className of metadataProvider.getInsertableClassNames()) {
				if (!COMMON_INSTANCES.has(className)) {
					restItems.push(new InstanceInsertItem(className));
				}
			}
			restItems.sort((left, right) => {
				return left.label.localeCompare(right.label);
			});
			for (const item of restItems.values()) {
				items.push(item);
			}
		}
	}
	await Promise.all(
		items.map((item) => item.updateIcon(settingsProvider, iconsProvider))
	);
	const className =
		typeof classNameOrInsertService === "string"
			? classNameOrInsertService
			: await vscode.window.showQuickPick(items);
	if (className) {
		const chosen =
			typeof className !== "string"
				? className instanceof InstanceInsertItem
					? className.className
					: className.label
				: className;
		const instanceName = await vscode.window.showInputBox({
			prompt: `Enter a name for the new ${chosen}`,
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
					instanceName,
					classNameOrInsertService === true
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
	filePath: string | null,
	className: string | undefined | null | void
): Promise<[boolean, RenameResult | undefined]> => {
	const instanceName = await vscode.window.showInputBox({
		prompt: `Enter a new name for the ${className ?? "Instance"}`,
		value: "",
	});
	if (instanceName) {
		try {
			return await renameExistingInstance(
				folderPath,
				filePath,
				instanceName
			);
		} catch (e) {
			vscode.window.showWarningMessage(
				`Failed to rename instance!\n\nError message:\n\n${e}`
			);
		}
	}
	return [false, undefined];
};

class InstanceInsertSeparator implements vscode.QuickPickItem {
	label = "";
	kind = vscode.QuickPickItemKind.Separator;

	async updateIcon(
		settingsProvider: SettingsProvider,
		iconsProvider: IconsProvider
	) {}
}

class InstanceInsertItem implements vscode.QuickPickItem {
	label: string;
	description?: string;
	iconPath?:
		| vscode.Uri
		| { light: vscode.Uri; dark: vscode.Uri }
		| vscode.ThemeIcon
		| undefined;

	constructor(public readonly className: string) {
		this.label = className;
		// TODO: Descriptions?
		if (className === "Folder") {
		} else if (className === "ModuleScript") {
		} else if (className === "LocalScript") {
		} else if (className === "Script") {
		}
	}

	async updateIcon(
		settingsProvider: SettingsProvider,
		iconsProvider: IconsProvider
	) {
		this.iconPath = iconsProvider.getClassIcon(
			settingsProvider.get("explorer.iconPack"),
			this.className
		);
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
