import * as vscode from "vscode"

import { DomInstance } from "../../server"

import { InsertInstanceItem, InsertInstanceSeparator } from "./item"
import { Providers } from ".."

type InsertMode = "None" | "Default" | "Services"
type InsertItem = InsertInstanceItem | InsertInstanceSeparator

export class InsertInstanceProvider implements vscode.Disposable {
	private readonly picker: vscode.QuickPick<InsertItem>
	private readonly input: vscode.InputBox
	private readonly disposables: vscode.Disposable[] = []

	private currentInsertMode: InsertMode = "None"
	private currentWorkspacePath: string | null = null
	private currentClassName: string | null = null
	private currentInstance: DomInstance | null = null

	constructor(public readonly providers: Providers) {
		this.picker = vscode.window.createQuickPick()
		this.picker.canSelectMany = false
		this.picker.placeholder = ""
		this.picker.title = "Insert Object"

		this.input = vscode.window.createInputBox()
		this.input.placeholder = "Enter a name for the new Instance"
		this.input.title = "Insert Object"

		this.disposables.push(this.picker.onDidChangeValue(() => this.updatePicker()))
		this.disposables.push(this.picker.onDidAccept(() => this.acceptPicker()))
		this.disposables.push(this.picker.onDidHide(() => this.hidePicker()))

		this.disposables.push(this.input.onDidChangeValue(() => this.updateInput()))
		this.disposables.push(this.input.onDidAccept(() => this.acceptInput()))
		this.disposables.push(this.input.onDidHide(() => this.hideInput()))
	}

	dispose() {
		this.picker.dispose()
		this.input.dispose()
		for (const disposable of this.disposables) {
			disposable.dispose()
		}
	}

	private async updatePicker() {
		if (!this.currentWorkspacePath || !this.currentInstance) {
			this.picker.items = []
			return
		}

		const newItems = new Array<InsertItem>()

		if (this.currentInsertMode !== "None") {
			const insertableClasses = this.providers.metadata.getInsertableClasses(
				this.currentInstance.className,
				this.currentInsertMode === "Services"
			)

			let currentGrouping = ""
			for (const insertableClass of insertableClasses) {
				const insertableGrouping =
					insertableClass.dialogCategoryOverride ??
					insertableClass.dialogCategory ??
					"Uncategorized"

				// Insert a separator between each grouping of classes, these should
				// already be sorted by getInsertableClasses so we get clear separators
				if (currentGrouping !== insertableGrouping) {
					currentGrouping = insertableGrouping
					newItems.push(
						new InsertInstanceSeparator(
							this.providers,
							this.currentWorkspacePath,
							insertableGrouping
						)
					)
				}

				newItems.push(
					new InsertInstanceItem(
						this.providers,
						this.currentWorkspacePath,
						insertableClass.className
					)
				)
			}
		}

		this.picker.items = newItems
	}

	private async updateInput() {
		if (this.currentWorkspacePath && this.currentInstance) {
			if (hasDisallowedCharacter(this.input.value)) {
				this.input.validationMessage =
					"Name must only contain alphanumeric characters, underscores, and dashes"
			} else {
				this.input.validationMessage = undefined
			}
		}
	}

	private async acceptPicker() {
		if (this.currentWorkspacePath && this.currentInstance) {
			const acceptedItem: InsertItem | undefined = this.picker.selectedItems[0]
			if (acceptedItem && acceptedItem instanceof InsertInstanceItem) {
				if (this.currentInsertMode === "Services") {
					// NOTE: We enforce services that have the same name as class
					// name, so just set the input and accept it manually right away
					this.currentClassName = acceptedItem.className
					this.input.value = acceptedItem.className
					this.acceptInput()
				} else {
					this.currentClassName = acceptedItem.className
					this.hidePicker()
					this.showInput()
				}
				return
			}
		}
		this.hideAndReset()
	}

	private async acceptInput() {
		if (this.currentWorkspacePath && this.currentInstance && this.currentClassName) {
			if (this.input.value && !this.input.validationMessage) {
				// Try to create the new instance with the chosen class name & name
				const result = await this.providers.explorerTree.insertInstance(
					this.currentWorkspacePath,
					this.currentInstance.id,
					this.currentClassName,
					this.input.value
				)
				if (result) {
					// ... also try to reveal the new instance
					await this.providers.explorerTree.revealById(
						this.currentWorkspacePath,
						result.id,
						true
					)
					// ... and open its corresponding file, if it is openable
					const canOpen = !!result.metadata?.actions?.canOpen
					const filePath = result.metadata?.paths?.file
					if (canOpen && filePath) {
						await vscode.commands.executeCommand(
							"vscode.open",
							vscode.Uri.file(filePath)
						)
					}
				}
			}
		}
		this.hideAndReset()
	}

	private showPicker() {
		this.picker.value = ""
		this.picker.items = []
		this.picker.show()
	}

	private showInput() {
		this.input.value = ""
		this.input.validationMessage = undefined
		if (this.currentClassName) {
			this.input.placeholder = `Enter a name for the new ${this.currentClassName}`
		} else {
			this.input.placeholder = "Enter a name for the new Instance"
		}
		this.input.show()
	}

	private hidePicker() {
		this.picker.hide()
		this.picker.value = ""
		this.picker.items = []
	}

	private hideInput() {
		this.input.hide()
		this.input.value = ""
		this.input.validationMessage = undefined
	}

	private hideAndReset() {
		this.hidePicker()
		this.hideInput()
		this.currentInsertMode = "None"
		this.currentWorkspacePath = null
		this.currentClassName = null
		this.currentInstance = null
	}

	public show(
		workspacePath: string,
		instance: DomInstance,
		predeterminedClassName?: string | null,
		insertService?: boolean | null
	) {
		this.hideAndReset()

		this.currentInsertMode = insertService ? "Services" : "Default"
		this.currentWorkspacePath = workspacePath
		this.currentInstance = instance

		this.input.title = insertService ? "Insert Service" : "Insert Instance"
		this.picker.title = insertService ? "Insert Service" : "Insert Instance"

		if (predeterminedClassName) {
			this.currentClassName = predeterminedClassName
			this.showInput()
			this.updateInput()
		} else {
			this.currentClassName = null
			this.showPicker()
			this.updatePicker()
		}
	}

	public hide() {
		this.hideAndReset()
	}
}

const hasDisallowedCharacter = (s: string): boolean => {
	return /[^a-zA-Z0-9_-]/.test(s)
}
