import * as vscode from "vscode";

import axios from "axios";

import { RobloxApiDump, parseApiDumpFromObject } from "./robloxApiDump";
import {
	RobloxReflectionMetadata,
	parseReflectionMetadataFromRobloxStudioZip,
} from "./robloxReflectionMetadata";

export { RobloxApiDump } from "./robloxApiDump";
export { RobloxReflectionMetadata } from "./robloxReflectionMetadata";

const URL_VERSION = "https://setup.rbxcdn.com/versionQTStudio";
const URL_API_DUMP = "https://setup.rbxcdn.com/%V-API-Dump.json";
const URL_STUDIO = "https://setup.rbxcdn.com/%V-RobloxStudio.zip";

const CACHE_KEY_VERSION = "roblox-version-hash";
const CACHE_PREFIX_API_DUMP = "roblox-api-dump";
const CACHE_PREFIX_REFLECTION = "roblox-reflection";

const getRobloxApiVersion = async (
	progressCallback: (progress: number) => any
): Promise<string> => {
	progressCallback(0);
	const result = await (new Promise((resolve, reject) => {
		axios
			.get(URL_VERSION, {
				onDownloadProgress(progressEvent) {
					if (progressEvent.progress) {
						progressCallback(progressEvent.progress);
					} else {
						progressCallback(0);
					}
				},
			})
			.then((res) => resolve(res.data))
			.catch((err) => reject(err));
	}) as Promise<string>);
	progressCallback(1);
	return result;
};

const getRobloxApiDump = async (
	version: string,
	progressCallback: (progress: number) => any
): Promise<RobloxApiDump> => {
	progressCallback(0);
	const result = await (new Promise((resolve, reject) => {
		axios
			.get(URL_API_DUMP.replace("%V", version), {
				onDownloadProgress(progressEvent) {
					if (progressEvent.progress) {
						progressCallback(progressEvent.progress * 0.9);
					} else {
						progressCallback(0);
					}
				},
			})
			.then((res) => {
				progressCallback(0.95);
				return parseApiDumpFromObject(res.data);
			})
			.then((apiDump) => resolve(apiDump))
			.catch((err) => reject(err));
	}) as Promise<RobloxApiDump>);
	progressCallback(1);
	return result;
};

const getRobloxApiReflection = async (
	version: string,
	progressCallback: (progress: number) => any
): Promise<RobloxReflectionMetadata> => {
	progressCallback(0);
	const result = await (new Promise((resolve, reject) => {
		axios
			.get(URL_STUDIO.replace("%V", version), {
				responseType: "arraybuffer",
				onDownloadProgress(progressEvent) {
					if (progressEvent.progress) {
						progressCallback(progressEvent.progress * 0.8);
					} else {
						progressCallback(0);
					}
				},
			})
			.then((res) => {
				progressCallback(0.85);
				return Buffer.from(res.data, "binary");
			})
			.then((buf) => {
				progressCallback(0.9);
				return parseReflectionMetadataFromRobloxStudioZip(buf);
			})
			.then((reflection) => resolve(reflection))
			.catch((err) => reject(err));
	}) as Promise<RobloxReflectionMetadata>);
	progressCallback(1);
	return result;
};

export const getRobloxCache = (context: vscode.ExtensionContext) => {
	const state = {
		hasVersion: false,
		hasApiDump: false,
		hasReflection: false,
		cachedVersion: undefined as undefined | string,
		cachedApiDump: undefined as undefined | RobloxApiDump,
		cachedReflection: undefined as undefined | RobloxReflectionMetadata,
	};
	for (const key of context.globalState.keys()) {
		if (key === CACHE_KEY_VERSION) {
			state.cachedVersion = context.globalState.get(key);
		} else if (key.startsWith(CACHE_PREFIX_API_DUMP)) {
			state.cachedApiDump = context.globalState.get(key);
		} else if (key.startsWith(CACHE_PREFIX_REFLECTION)) {
			state.cachedReflection = context.globalState.get(key);
		}
	}
	state.hasVersion = !!state.cachedVersion;
	state.hasApiDump = !!state.cachedApiDump;
	state.hasReflection = !!state.cachedReflection;
	return state;
};

export const setRobloxCache = (
	context: vscode.ExtensionContext,
	version: string,
	apiDump: RobloxApiDump,
	reflection: RobloxReflectionMetadata
) => {
	const keysToRemove = [];
	for (const key of context.globalState.keys()) {
		if (
			key.match(CACHE_KEY_VERSION) ||
			key.match(CACHE_PREFIX_API_DUMP) ||
			key.match(CACHE_PREFIX_REFLECTION)
		) {
			keysToRemove.push(key);
		}
	}
	for (const key of keysToRemove.values()) {
		context.globalState.update(key, undefined);
	}
	context.globalState.update(CACHE_KEY_VERSION, version);
	context.globalState.update(`${CACHE_PREFIX_API_DUMP}-${version}`, apiDump);
	context.globalState.update(
		`${CACHE_PREFIX_REFLECTION}-${version}`,
		reflection
	);
};

export const initRobloxCache = async (context: vscode.ExtensionContext) => {
	const dev = context.extensionMode === vscode.ExtensionMode.Development;
	const cache = getRobloxCache(context);
	if (dev || !cache.hasVersion || !cache.hasApiDump || !cache.hasReflection) {
		// No roblox cache was found or it was partially
		// missing, treat it as if the cache does not exist
		await vscode.window.withProgress(
			{
				title: "Downloading Roblox API...\n\n",
				location: vscode.ProgressLocation.Notification,
			},
			async (indicator) => {
				try {
					const apiVersion = await getRobloxApiVersion((progress) => {
						indicator.report({
							increment: Math.round(progress * 20),
						});
					});

					// NOTE: The progress bar is weighted heavily towards showing changes
					// in downloading the API dump & reflection since those can be quite
					// large and users with a slow connection will get more value there
					let progressApiDump = 0;
					let progressReflection = 0;
					const updateProgress = () => {
						indicator.report({
							increment: Math.round(
								20 +
									40 * progressApiDump +
									40 * progressReflection
							),
						});
					};

					const [apiDump, apiReflection] = await Promise.all([
						getRobloxApiDump(apiVersion, (progress) => {
							progressApiDump = progress;
							updateProgress();
						}),
						getRobloxApiReflection(apiVersion, (progress) => {
							progressReflection = progress;
							updateProgress();
						}),
					]);

					indicator.report({ increment: 100 });
					setRobloxCache(context, apiVersion, apiDump, apiReflection);
				} catch (err) {
					vscode.window.showErrorMessage(
						`Failed to download Roblox API!\n\n${err}`
					);
				}
			}
		);
	} else {
		// A cache was found, we will silently check for a new version,
		// and if a new version was found, download new data with a progress
		// bar, if this fails we instead fall back to the already available cache
		try {
			const apiVersion = await getRobloxApiVersion(() => {});
			if (apiVersion !== cache.cachedVersion) {
				await vscode.window.withProgress(
					{
						title: "Downloading latest Roblox API...\n\n",
						location: vscode.ProgressLocation.Notification,
					},
					async (indicator) => {
						let progressApiDump = 0;
						let progressReflection = 0;
						const updateProgress = () => {
							indicator.report({
								increment: Math.round(
									50 * progressApiDump +
										50 * progressReflection
								),
							});
						};

						const [apiDump, apiReflection] = await Promise.all([
							getRobloxApiDump(apiVersion, (progress) => {
								progressApiDump = progress;
								updateProgress();
							}),
							getRobloxApiReflection(apiVersion, (progress) => {
								progressReflection = progress;
								updateProgress();
							}),
						]);

						indicator.report({ increment: 100 });
						setRobloxCache(
							context,
							apiVersion,
							apiDump,
							apiReflection
						);
					}
				);
			}
		} catch (err) {
			vscode.window.showErrorMessage(
				"Failed to download the latest Roblox API!" +
					"\n\nInstances may not appear correctly, but the extension will still work." +
					`\n\n${err}`
			);
		}
	}
	return getRobloxCache(context);
};
