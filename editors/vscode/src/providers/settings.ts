import * as vscode from "vscode";

import { IconPack } from "./icons";

const EXTENSION = "roblox-ui";
const DEFAULTS = {
	"explorer.showDataModel": true,
	"explorer.showClassNames": false,
	"explorer.showFilePaths": false,
	"explorer.iconPack": "Vanilla2" as IconPack,
	"sourcemap.autogenerate": true,
	"sourcemap.ignoreGlobs": [],
	"sourcemap.includeNonScripts": false,
	"sourcemap.rojoProjectFile": "default.project.json",
	"wally.modifyPackagesDir": true,
	"wally.showPackageVersion": true,
};

type Settings = typeof DEFAULTS;
type SettingsKey = keyof Settings;
type SettingsValue<K extends SettingsKey> = Settings[K];

// biome-ignore lint/suspicious/noExplicitAny:
type SettingsCallback<K extends SettingsKey> = (value: SettingsValue<K>) => any;

export class SettingsProvider implements vscode.Disposable {
	// biome-ignore lint/suspicious/noExplicitAny:
	private readonly values: Map<string, any> = new Map();
	// biome-ignore lint/suspicious/noExplicitAny:
	private readonly events: Map<string, vscode.EventEmitter<any>> = new Map();
	private readonly disposable: vscode.Disposable;

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
			if (this.values.get(key) !== initialValue && initialValue !== undefined) {
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
						this.events.get(key)?.fire(value);
					}
				}
			}
		});
	}

	/**
	 * Get the current value for a given setting.
	 */
	public get<K extends SettingsKey>(key: K): SettingsValue<K> {
		const value = this.values.get(key);
		if (value === undefined) {
			throw new Error(`Missing default value for setting "${key}"`);
		}
		return value;
	}

	/**
	 * Listens for the given setting changing and runs the callback.
	 *
	 * This will run the callback once initially with the current value.
	 */
	public listen<K extends SettingsKey>(key: K, callback: SettingsCallback<K>): vscode.Disposable {
		const initialValue = this.values.get(key);
		if (initialValue === undefined) {
			throw new Error(`Missing initial setting value for setting "${key}"`);
		}
		callback(initialValue);
		const event = this.events.get(key);
		if (event === undefined) {
			throw new Error(`Missing event for setting "${key}"`);
		}
		return event.event(callback);
	}

	dispose() {
		for (const event of this.events.values()) {
			event.dispose();
		}
		this.disposable.dispose();
	}
}
