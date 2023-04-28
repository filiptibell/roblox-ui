import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs/promises";

const workspaceFs = vscode.workspace.fs;

import { IconPack, downloadIconPack } from "../web/icons";
import { RobloxApiDump, RobloxReflectionMetadata } from "../web/roblox";

type IconPackIcon = { light: vscode.Uri; dark: vscode.Uri };
type IconPackData = Map<string, IconPackIcon>;

const stripFileExt = (filePath: string): string => {
	const ext = path.extname(filePath);
	if (ext) {
		return filePath.slice(0, filePath.length - ext.length);
	} else {
		return filePath;
	}
};

const readDir = async (filePath: string): Promise<string[] | null> => {
	const fileUri = vscode.Uri.file(filePath);
	try {
		const entries = await workspaceFs.readDirectory(fileUri);
		return [...entries.map((entry) => entry[0])];
	} catch {
		return null;
	}
};

export class IconsProvider implements vscode.Disposable {
	private packs: Map<IconPack, IconPackData> = new Map();

	constructor(
		private readonly context: vscode.ExtensionContext,
		private readonly apiDump: RobloxApiDump,
		private readonly reflection: RobloxReflectionMetadata
	) {}

	private async tryGetCachedIcons(
		pack: IconPack
	): Promise<IconPackData | null> {
		const storagePath = path.join(
			this.context.globalStorageUri.fsPath,
			"iconPacks",
			pack
		);
		const storageDirs = await readDir(storagePath);
		if (storageDirs && storageDirs.length >= 2) {
			if (
				storageDirs.find((entry) => entry === "light") &&
				storageDirs.find((entry) => entry === "dark")
			) {
				const packData = new Map();

				// Read all entries in the file icon dirs to
				// know which class names we have icons made for
				const [filesLight, filesDark] = await Promise.all([
					readDir(path.join(storagePath, "light")),
					readDir(path.join(storagePath, "dark")),
				]);

				// Gather a set of all known filenames
				const fileNames = new Set<string>([]);
				for (const fileName of filesLight!.values()) {
					fileNames.add(fileName);
				}
				for (const fileName of filesDark!.values()) {
					fileNames.add(fileName);
				}

				// Save file icon datas with class name as key
				for (const fileName of fileNames.values()) {
					packData.set(stripFileExt(fileName), {
						light: vscode.Uri.file(
							path.join(storagePath, "light", fileName)
						),
						dark: vscode.Uri.file(
							path.join(storagePath, "dark", fileName)
						),
					});
				}

				// Save and return cached pack data
				this.packs.set(pack, packData);
				return packData;
			}
		}
		return null;
	}

	private async getOrDownloadIconPack(pack: IconPack): Promise<IconPackData> {
		// Return any in-memory cached pack data
		let packData = this.packs.get(pack);
		if (packData) {
			return packData;
		}

		// Check the extension cache directory for already downloaded files
		const cachedIconData =
			this.context.extensionMode !== vscode.ExtensionMode.Development
				? await this.tryGetCachedIcons(pack)
				: null;
		if (cachedIconData) {
			return cachedIconData;
		}

		// Not cached, we need to download the pack
		const downloadedIcons = await downloadIconPack(
			pack,
			this.apiDump,
			this.reflection
		);

		// Create the directories where we will store our icons
		const storagePath = path.join(
			this.context.globalStorageUri.fsPath,
			"iconPacks",
			pack
		);
		const dirLight = path.join(storagePath, "light");
		const dirDark = path.join(storagePath, "dark");
		await Promise.all([
			workspaceFs.createDirectory(vscode.Uri.file(dirLight)),
			workspaceFs.createDirectory(vscode.Uri.file(dirDark)),
		]);

		const fsDirLight = path.join(
			vscode.workspace.workspaceFolders![0]!.uri.fsPath,
			"iconPacks",
			pack,
			"light"
		);
		const fsDirDark = path.join(
			vscode.workspace.workspaceFolders![0]!.uri.fsPath,
			"iconPacks",
			pack,
			"dark"
		);
		await Promise.all([
			fs.mkdir(fsDirDark, { recursive: true }),
			fs.mkdir(fsDirLight, { recursive: true }),
		]);

		// Save all of the icons to files for caching and usage
		packData = new Map();
		const filePromises = new Array<Thenable<void>>();
		for (const [fileName, fileIcons] of downloadedIcons.entries()) {
			const uriLight = vscode.Uri.file(path.join(dirLight, fileName));
			const uriDark = vscode.Uri.file(path.join(dirDark, fileName));
			filePromises.push(workspaceFs.writeFile(uriLight, fileIcons.light));
			filePromises.push(workspaceFs.writeFile(uriDark, fileIcons.dark));
			fs.writeFile(path.join(fsDirLight, fileName), fileIcons.light);
			fs.writeFile(path.join(fsDirDark, fileName), fileIcons.dark);
			packData.set(stripFileExt(fileName), {
				light: uriLight,
				dark: uriDark,
			});
		}
		await Promise.all(filePromises);

		// Save and return new pack data
		this.packs.set(pack, packData);
		return packData;
	}

	/**
	 * Get icon pack data for the given icon pack, and download the icon pack if necessary.
	 */
	public async getPack(pack: IconPack): Promise<IconPackData> {
		return await this.getOrDownloadIconPack(pack);
	}

	/**
	 * Get file paths for the given icon name.
	 *
	 * If the icon pack is not cached in memory or has not been
	 * downloaded, this will download and cache the icon pack.
	 *
	 * This may return `undefined` if the icon with the given name does not exist.
	 */
	public async getPackIcon(
		pack: IconPack,
		name: string
	): Promise<IconPackIcon | undefined> {
		const packData = await this.getOrDownloadIconPack(pack);
		return packData.get(name) ?? undefined;
	}

	/**
	 * Get file paths for the given instance class name.
	 *
	 * If the icon pack is not cached in memory or has not been
	 * downloaded, this will download and cache the icon pack.
	 *
	 * This will properly handle relationships of subclasses and superclasses,
	 * meaning that if an instance icon does not exist for a class but exists for a
	 * superclass of that class, the icon for the superclass will be returned instead.
	 *
	 * Note that if an icon pack does not have an icon for either `Instance` or `Unknown`,
	 * this method may return `undefined` if no icon for the class name was found.
	 */
	public async getPackIconForClassName(
		pack: IconPack,
		className: string
	): Promise<IconPackIcon | undefined> {
		const packData = await this.getOrDownloadIconPack(pack);

		let current: string | null = className;
		while (current && current.length > 0) {
			const icon = packData.get(current);
			if (icon !== undefined) {
				return icon;
			} else {
				const apiClass = this.apiDump.Classes.get(current);
				if (apiClass && apiClass.Superclass) {
					current = apiClass.Superclass;
				} else {
					break;
				}
			}
		}

		return packData.get("Instance") ?? packData.get("Unknown") ?? undefined;
	}

	dispose() {}
}
