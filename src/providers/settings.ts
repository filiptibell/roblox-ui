import * as vscode from "vscode";

const EXTENSION = "rojoViewer";
const DEFAULTS = {
	autogenerateSourcemap: true,
	ignoreGlobs: ["**/_Index/**"],
	includeNonScripts: false,
	showClassNames: false,
	showFilePaths: false,
	rojoProjectFile: "default.project.json",
};

type Settings = typeof DEFAULTS;
type SettingsKey = keyof Settings;
type SettingsValue<K extends SettingsKey> = Settings[K];
type SettingsCallback<K extends SettingsKey> = (value: SettingsValue<K>) => any;

export class SettingsProvider implements vscode.Disposable {
	private values: Map<string, any> = new Map();
	private events: Map<string, vscode.EventEmitter<any>> = new Map();
	private disposable: vscode.Disposable;

	constructor() {
		// Add in defaults as current values
		for (const [key, value] of Object.entries(DEFAULTS)) {
			this.values.set(key, value);
			this.events.set(key, new vscode.EventEmitter());
		}

		// Add in current settings values
		const initialConfig = vscode.workspace.getConfiguration(EXTENSION);
		for (const key of Object.keys(DEFAULTS)) {
			const initialValue = initialConfig.get(key);
			if (
				this.values.get(key) !== initialValue &&
				initialValue !== undefined
			) {
				this.values.set(key, initialValue);
			}
		}

		// Listen for changes
		this.disposable = vscode.workspace.onDidChangeConfiguration((event) => {
			const changes: Array<string> = new Array();
			for (const key of this.values.keys()) {
				if (event.affectsConfiguration(`${EXTENSION}.${key}`)) {
					changes.push(key);
				}
			}
			if (changes.length > 0) {
				const config = vscode.workspace.getConfiguration(EXTENSION);
				for (const key of changes) {
					const value = config.get(key);
					if (this.values.get(key) !== value) {
						this.values.set(key, value);
						this.events.get(key)!.fire(value);
					}
				}
			}
		});
	}

	/**
	 * Get the current value for a given setting.
	 */
	public get<K extends SettingsKey>(key: K): SettingsValue<K> {
		return this.values.get(key)!;
	}

	/**
	 * Listens for the given setting changing and runs the callback.
	 *
	 * This will run the callback once initially with the current value.
	 */
	public listen<K extends SettingsKey>(
		key: K,
		callback: SettingsCallback<K>
	): vscode.Disposable {
		callback(this.values.get(key)!);
		return this.events.get(key)!.event(callback);
	}

	dispose() {
		for (const event of this.events.values()) {
			event.dispose();
		}
		this.disposable.dispose();
	}
}
