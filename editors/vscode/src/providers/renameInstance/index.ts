import * as vscode from "vscode"

import { DomInstance } from "../../server"
import { Providers } from ".."

export class RenameInstanceProvider implements vscode.Disposable {
	private readonly input: vscode.InputBox
	private readonly disposables: vscode.Disposable[] = []

	private currentWorkspacePath: string | null = null
	private currentInstance: DomInstance | null = null

	constructor(public readonly providers: Providers) {
		this.input = vscode.window.createInputBox()
		this.input.placeholder = "Enter a new name..."
		this.input.title = "Rename Instance"

		const onChange = () => this.update()
		const onAccept = () => this.accept()
		const onHide = () => this.hide()

		this.disposables.push(this.input.onDidChangeValue(onChange))
		this.disposables.push(this.input.onDidAccept(onAccept))
		this.disposables.push(this.input.onDidHide(onHide))
	}

	dispose() {
		this.input.dispose()
		for (const disposable of this.disposables) {
			disposable.dispose()
		}
	}

	private async update() {
		if (this.currentWorkspacePath && this.currentInstance) {
			if (hasDisallowedCharacter(this.input.value)) {
				this.input.validationMessage =
					'Name must only contain alphanumeric characters, or one of the following characters: ".", "_", "-", " "'
			} else {
				this.input.validationMessage = undefined
			}
		}
	}

	private async accept() {
		if (
			this.currentWorkspacePath &&
			this.currentInstance &&
			this.input.value &&
			!this.input.validationMessage
		) {
			// Try to rename the instance to the chosen name
			const renamed = await this.providers.explorerTree.renameInstance(
				this.currentWorkspacePath,
				this.currentInstance.id,
				this.input.value
			)
			if (renamed) {
				// ... also try to reveal the instance
				await this.providers.explorerTree.revealById(
					this.currentWorkspacePath,
					this.currentInstance.id,
					true
				)
			}
		}
		this.hide()
	}

	public show(workspacePath: string, instance: DomInstance) {
		this.currentWorkspacePath = workspacePath
		this.currentInstance = instance
		this.input.validationMessage = undefined
		this.input.value = ""
		this.input.show()
		this.update()
	}

	public hide() {
		this.input.hide()
		this.input.value = ""
		this.input.validationMessage = undefined
		this.currentInstance = null
		this.currentWorkspacePath = null
	}
}

const hasDisallowedCharacter = (s: string): boolean => {
	return /[^a-zA-Z0-9_. -]/.test(s)
}
