import * as vscode from "vscode"
import * as path from "path"
import * as fs from "fs"

import { Providers } from ".."

import type { IconPack, IconPackData, IconPackIcon, IconPackMetadatas } from "./types"
import { runCommand } from "../../server/child"
export type { IconPack } from "./types"

const CUSTOM_ICON_PACK: IconPack = "RobloxCustom"

const getAllIconPacks = (): Array<IconPack> => {
	return ["None", "Classic", "Vanilla2"]
}

const getBasePath = (extensionPath: string, pack: IconPack): string => {
	return path.join(extensionPath, "out", "icons", pack)
}

const readIconPackMetadatas = (extensionPath: string, pack: IconPack): IconPackMetadatas => {
	if (pack === "None") {
		return {
			light: {
				classCount: 0,
				classIcons: {},
			},
			dark: {
				classCount: 0,
				classIcons: {},
			},
		}
	}

	const metaPathLight = path.join(getBasePath(extensionPath, pack), "light", "metadata.json")
	const metaPathDark = path.join(getBasePath(extensionPath, pack), "dark", "metadata.json")

	const metaContentsLight = fs.readFileSync(metaPathLight, "utf-8")
	const metaContentsDark = fs.readFileSync(metaPathDark, "utf-8")

	return {
		light: JSON.parse(metaContentsLight),
		dark: JSON.parse(metaContentsDark),
	}
}

const createIconPackData = (
	extensionPath: string,
	pack: IconPack,
	metas: IconPackMetadatas
): IconPackData => {
	const icons = new Map<string, IconPackIcon>()

	const allClassNames = new Set<string>()
	for (const className of Object.keys(metas.light.classIcons)) {
		allClassNames.add(className)
	}
	for (const className of Object.keys(metas.dark.classIcons)) {
		allClassNames.add(className)
	}

	for (const className of allClassNames) {
		const iconPathLight = path.join(
			getBasePath(extensionPath, pack),
			"light",
			metas.light.classIcons[className]
		)
		const iconPathDark = path.join(
			getBasePath(extensionPath, pack),
			"dark",
			metas.dark.classIcons[className]
		)
		icons.set(className, {
			light: vscode.Uri.file(iconPathLight),
			dark: vscode.Uri.file(iconPathDark),
		})
	}

	return icons
}

export class IconsProvider implements vscode.Disposable {
	private readonly metas: Map<IconPack, IconPackMetadatas> = new Map()
	private readonly icons: Map<IconPack, IconPackData> = new Map()

	private customIconsLoading = false
	private customIconsErrored = false

	private readonly _onDidChangeCustomIcons: vscode.EventEmitter<void> = new vscode.EventEmitter()
	public readonly onDidChangeCustomIcons: vscode.Event<void> = this._onDidChangeCustomIcons.event

	private readonly disposables: vscode.Disposable[] = []

	constructor(public readonly providers: Providers) {
		for (const pack of getAllIconPacks()) {
			const metas = readIconPackMetadatas(providers.extensionContext.extensionPath, pack)
			const icons = createIconPackData(providers.extensionContext.extensionPath, pack, metas)
			this.metas.set(pack, metas)
			this.icons.set(pack, icons)
		}
		const updateCustomIconDir = () => {
			const customIconDir = providers.settings.get("explorer.customIconDir")
			if (customIconDir && customIconDir.trim().length > 0) {
				this.customIconsLoading = true
				this.customIconsErrored = false

				const outputPath = getBasePath(
					providers.extensionContext.extensionPath,
					CUSTOM_ICON_PACK
				)
				const commandArgs = [
					"generate-icons",
					"--input",
					customIconDir,
					"--output",
					outputPath,
				]

				runCommand(providers, commandArgs)
					.then(() => {
						const metas = readIconPackMetadatas(
							providers.extensionContext.extensionPath,
							CUSTOM_ICON_PACK
						)
						const icons = createIconPackData(
							providers.extensionContext.extensionPath,
							CUSTOM_ICON_PACK,
							metas
						)
						this.metas.set(CUSTOM_ICON_PACK, metas)
						this.icons.set(CUSTOM_ICON_PACK, icons)

						this.customIconsLoading = false
						this.customIconsErrored = false

						this._onDidChangeCustomIcons.fire()
					})
					.catch((e) => {
						this.customIconsLoading = false
						this.customIconsErrored = true
						vscode.window.showErrorMessage(`Failed to read custom icon pack: ${e}`)
					})
			} else {
				this.metas.delete(CUSTOM_ICON_PACK)
				this.icons.delete(CUSTOM_ICON_PACK)
			}
		}
		this.disposables.push(
			providers.settings.listen("explorer.customIconDir", updateCustomIconDir)
		)
		updateCustomIconDir()
	}

	public getClassIcon(className: string): IconPackIcon | undefined {
		const shouldUseNormalIcons =
			this.customIconsLoading ||
			this.customIconsErrored ||
			!this.providers.settings.get("explorer.customIconDir")
		const pack = shouldUseNormalIcons
			? this.providers.settings.get("explorer.iconPack")
			: CUSTOM_ICON_PACK
		const icon = this.icons.get(pack)?.get(className) ?? undefined
		return icon
	}

	dispose() {
		for (const disposable of this.disposables) {
			disposable.dispose()
		}
		this._onDidChangeCustomIcons.dispose()
	}
}
