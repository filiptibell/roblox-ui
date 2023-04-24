import * as vscode from "vscode";

import axios from "axios";

import { RobloxApiDump, parseApiDumpFromObject } from "./robloxApiDump";
import {
	RobloxReflectionMetadata,
	parseReflectionMetadataFromRobloxStudioZip,
} from "./robloxReflectionMetadata";

const URL_VERSION = "https://setup.rbxcdn.com/versionQTStudio";
const URL_API_DUMP = "https://setup.rbxcdn.com/%V-API-Dump.json";
const URL_STUDIO = "https://setup.rbxcdn.com/%V-RobloxStudio.zip";

const CACHE_KEY_VERSION = "roblox-version-hash";
const CACHE_PREFIX_API_DUMP = "roblox-api-dump";
const CACHE_PREFIX_REFLECTION = "roblox-reflection";

export async function getRobloxApiVersion(
	context: vscode.ExtensionContext
): Promise<string> {
	return new Promise((resolve, reject) => {
		axios
			.get(URL_VERSION)
			.then((res) => {
				context.globalState.update(CACHE_KEY_VERSION, res.data);
				resolve(res.data);
			})
			.catch((err) => {
				const cached: string | undefined =
					context.globalState.get(CACHE_KEY_VERSION);
				if (cached) {
					vscode.window.showWarningMessage(
						"Failed to fetch latest Roblox version!" +
							"\nRojo Explorer will use a cached version, which may be out of date."
					);
					resolve(cached);
				} else {
					vscode.window.showWarningMessage(
						`Failed to fetch latest Roblox version!\n${err}`
					);
					reject(err);
				}
			});
	});
}

export async function getRobloxApiDump(
	context: vscode.ExtensionContext,
	verArg: string | void
): Promise<RobloxApiDump> {
	let ver = "";
	if (!verArg) {
		ver = await getRobloxApiVersion(context);
	} else {
		ver = verArg;
	}

	let cachedKey;
	for (const key of context.globalState.keys()) {
		if (key.startsWith(CACHE_PREFIX_API_DUMP)) {
			cachedKey = key;
			break;
		}
	}
	if (cachedKey) {
		if (cachedKey !== ver) {
			context.globalState.update(cachedKey, undefined);
		} else {
			const cachedValue: RobloxApiDump | undefined =
				context.globalState.get(ver);
			if (cachedValue) {
				return Promise.resolve(cachedValue);
			}
		}
	}

	return new Promise((resolve, reject) => {
		axios
			.get(URL_API_DUMP.replace("%V", ver))
			.then((res) => parseApiDumpFromObject(res.data))
			.then((apiDump) => {
				context.globalState.update(
					`${CACHE_PREFIX_API_DUMP}-${ver}`,
					apiDump
				);
				resolve(apiDump);
			})
			.catch((err) => {
				vscode.window.showWarningMessage(
					`Failed to fetch API dump!\n${err}`
				);
				reject(err);
			});
	});
}

export async function getRobloxApiReflection(
	context: vscode.ExtensionContext,
	verArg: string | void
): Promise<RobloxReflectionMetadata> {
	let ver = "";
	if (!verArg) {
		ver = await getRobloxApiVersion(context);
	} else {
		ver = verArg;
	}

	let cachedKey;
	for (const key of context.globalState.keys()) {
		if (key.startsWith(CACHE_PREFIX_REFLECTION)) {
			cachedKey = key;
			break;
		}
	}
	if (cachedKey) {
		if (cachedKey !== ver) {
			context.globalState.update(cachedKey, undefined);
		} else {
			const cachedValue: RobloxReflectionMetadata | undefined =
				context.globalState.get(ver);
			if (cachedValue) {
				return Promise.resolve(cachedValue);
			}
		}
	}

	return new Promise((resolve, reject) => {
		axios
			.get(URL_STUDIO.replace("%V", ver), {
				responseType: "arraybuffer",
			})
			.then((res) => Buffer.from(res.data, "binary"))
			.then((buf) => parseReflectionMetadataFromRobloxStudioZip(buf))
			.then((reflection) => {
				context.globalState.update(
					`${CACHE_PREFIX_REFLECTION}-${ver}`,
					reflection
				);
				resolve(reflection);
			})
			.catch((err) => {
				vscode.window.showWarningMessage(
					`Failed to fetch Reflection Metadata!\n${err}`
				);
				reject(err);
			});
	});
}
