import * as vscode from "vscode"
import * as path from "path"
import * as fs from "fs"

import { Providers } from ".."

import type { Classes, ClassData, Reflection } from "./types"

const COMMON_INSTANCES = new Array("ModuleScript", "LocalScript", "Script")

export type MetadataInsertableClass = {
	className: string
	isCommon: boolean
	isService: boolean
	isPreferred: boolean
	dialogCategoryOverride?: string
	dialogCategory?: string
}

export class MetadataProvider implements vscode.Disposable {
	private readonly classes: Classes
	private readonly reflection: Reflection

	constructor(public readonly providers: Providers) {
		const classesPath = path.join(
			providers.extensionContext.extensionPath,
			"out",
			"data",
			"classes.json"
		)
		const reflectionPath = path.join(
			providers.extensionContext.extensionPath,
			"out",
			"data",
			"reflection.json"
		)

		const classesContents = fs.readFileSync(classesPath, "utf-8")
		const reflectionContents = fs.readFileSync(reflectionPath, "utf-8")

		this.classes = JSON.parse(classesContents)
		this.reflection = JSON.parse(reflectionContents)
	}

	public getClassData(className: string): ClassData | null {
		const classData = this.classes.classDatas[className]
		if (typeof classData === "object") {
			return classData
		}
		return null
	}

	public getExplorerOrder(className: string): number | null {
		const classReflection = this.reflection.classes[className]
		if (typeof classReflection === "object") {
			const classOrder = classReflection.values?.ExplorerOrder
			if (typeof classOrder === "number") {
				return classOrder
			}
		}
		return null
	}

	public getInsertableClasses(
		parentClassName: string,
		servicesOnly: boolean
	): Array<MetadataInsertableClass> {
		const insertableClasses = new Array<MetadataInsertableClass>()

		for (const [className, classData] of Object.entries(this.classes.classDatas)) {
			if (classData.notCreatable) {
				continue
			}

			const reflectionData = this.reflection.classes[className]

			const isCommon = COMMON_INSTANCES.indexOf(className) >= 0
			const isService = !!classData.isService
			const isPreferred = reflectionData?.values?.PreferredParent === parentClassName
			if (isService && !servicesOnly) {
				continue
			}

			const dialogCategoryOverride = isCommon
				? "Common"
				: isPreferred
				? "Preferred"
				: isService
				? "Services"
				: undefined
			let dialogCategory = reflectionData?.values?.ClassCategory
			if (typeof dialogCategory !== "string") {
				dialogCategory = undefined
			}

			insertableClasses.push({
				className,
				isCommon,
				isService,
				isPreferred,
				dialogCategoryOverride,
				dialogCategory,
			})
		}

		insertableClasses.sort((a, b) => {
			if (a.isCommon !== b.isCommon) {
				return a.isCommon ? -1 : 1 // Common instances first
			}
			if (a.isCommon && b.isCommon) {
				const indexA = COMMON_INSTANCES.indexOf(a.className)
				const indexB = COMMON_INSTANCES.indexOf(b.className)
				return indexA < indexB ? -1 : 1 // Preserve order of common instances array
			}

			if (a.isService !== b.isService) {
				return a.isService ? -1 : 1 // Services after
			}

			if (a.isPreferred !== b.isPreferred) {
				return a.isPreferred ? -1 : 1 // Preferred after
			}

			if (!!a.dialogCategory !== !!b.dialogCategory) {
				return a.dialogCategory ? -1 : 1 // Things that have proper categories first
			}
			if (a.dialogCategory && b.dialogCategory && a.dialogCategory !== b.dialogCategory) {
				return a.dialogCategory.localeCompare(b.dialogCategory) // Different categories, alphabetically
			}

			return a.className.localeCompare(b.className) // Anything else alphabetically
		})

		return insertableClasses
	}

	dispose() {}
}
