import * as vscode from "vscode";
import * as path from "path";

const anymatch = require("anymatch");

import { SettingsProvider } from "../providers/settings";
import { RojoTreeProvider } from "../providers/explorer";

import { isInitFilePath, isBinaryFilePath } from "./rojo";
import {
	findPackageSource,
	findPackageSourceNode,
	parseWallySpec,
} from "./wally";
import { MetadataProvider } from "../providers/metadata";
import { RpcMessage, startServer } from "./server";

const PACKAGE_CLASS_NAME = "Package";

export type SourcemapNode = {
	name: string;
	className: string;
	folderPath?: string;
	filePaths?: string[];
	children?: SourcemapNode[];
	wallyVersion?: string;
};

const postprocessSourcemapNode = async (
	workspacePath: string,
	matcher: any | null,
	node: SourcemapNode,
	parent?: SourcemapNode | void,
	modifyWally?: boolean | void
): Promise<SourcemapNode | null> => {
	// Init files have a guaranteed parent directory
	if (!node.folderPath && node.filePaths) {
		for (const filePath of node.filePaths.values()) {
			if (isInitFilePath(filePath)) {
				node.folderPath = path.dirname(filePath);
			}
		}
	}

	// Otherwise we look at the parent of this node and try to
	// join its folder path with this sourcemap node folder name
	if (!node.folderPath && node.className === "Folder") {
		if (parent && parent.folderPath) {
			node.folderPath = path.join(parent.folderPath, node.name);
		}
	}

	// Check if we should keep this sourcemap node using the current glob sets,
	// making sure to always keep at least the root node (node without parent)
	if (parent && matcher) {
		if (node.folderPath && matcher(node.folderPath)) {
			return null;
		} else if (node.filePaths) {
			for (const filePath of node.filePaths.values()) {
				if (matcher(filePath)) {
					return null;
				}
			}
		}
	}

	// Check if we are in a Wally packages dir and modify it, when desired
	if (modifyWally && node.className === "Folder" && node.children) {
		if (node.name === "_Index") {
			return null;
		}
		let wallyIndexNode: SourcemapNode | undefined;
		for (const child of node.children.values()) {
			if (child.name === "_Index") {
				wallyIndexNode = child;
				break;
			}
		}
		if (wallyIndexNode) {
			// Turns packages folder children into "package sources"
			const childPromises = node.children
				.filter((child) => child !== wallyIndexNode)
				.map((child) => {
					if (child.filePaths && child.filePaths.length >= 1) {
						const fullPath = path.join(
							workspacePath,
							child.filePaths[0]
						);
						return findPackageSource(child.name, fullPath).then(
							(source) => {
								if (source) {
									return {
										child,
										source,
									};
								} else {
									return undefined;
								}
							}
						);
					} else {
						return Promise.resolve(undefined);
					}
				});
			// Map child package sources generated above to package instances
			for (const packageSource of await Promise.all(childPromises)) {
				if (packageSource) {
					const { child, source } = packageSource;
					const indexChild = findPackageSourceNode(
						wallyIndexNode,
						source
					);
					if (indexChild) {
						child.className = PACKAGE_CLASS_NAME;
						child.name = source.originalName;
						child.children = indexChild.children;
						child.filePaths = indexChild.filePaths;
						child.folderPath = indexChild.folderPath;
						const packageSpec = parseWallySpec(source.outerName);
						if (packageSpec) {
							child.wallyVersion = packageSpec.version;
						}
					}
				}
			}
		}
	}

	// Process children and remove them if ignore globs were matched
	if (node.children) {
		const childPromises: Promise<void>[] = [];
		const indicesToRemove: number[] = [];

		for (const [index, child] of node.children.entries()) {
			childPromises.push(
				postprocessSourcemapNode(
					workspacePath,
					matcher,
					child,
					node,
					modifyWally
				).then((node) => {
					if (!node) {
						child.name = "REMOVED";
						indicesToRemove.push(index);
					}
				})
			);
		}

		await Promise.allSettled(childPromises);

		for (const index of indicesToRemove.reverse()) {
			node.children.splice(index, 1);
		}
	}
	return node;
};

const postprocessSourcemap = async (
	workspacePath: string,
	settings: SettingsProvider,
	sourcemap: SourcemapNode
) => {
	const ignoreGlobs = settings.get("sourcemap.ignoreGlobs");
	const modifyWally = settings.get("wally.modifyPackagesDir");
	if (ignoreGlobs && ignoreGlobs.length > 0) {
		await postprocessSourcemapNode(
			workspacePath,
			anymatch(ignoreGlobs),
			sourcemap,
			undefined,
			modifyWally
		);
	} else {
		await postprocessSourcemapNode(
			workspacePath,
			null,
			sourcemap,
			undefined,
			modifyWally
		);
	}
};

const PRIMARY_FILE_PRIORITIES = new Map([
	["init.luau", 0],
	["init.lua", 0],
	["init.model.json", 1],
	["init.meta.json", 2],
	[".luau", 3],
	[".lua", 3],
	[".model.json", 4],
	[".meta.json", 5],
]);

const getSortKey = (path: string): number => {
	for (const [ext, value] of PRIMARY_FILE_PRIORITIES) {
		if (path.endsWith(ext)) {
			return value * 1000 + path.length;
		}
	}
	return Infinity;
};

export const findPrimaryFilePath = (
	node: SourcemapNode,
	allowBinaryFiles: boolean | void
): string | null => {
	if (node.filePaths) {
		// Sort and find using ordering - init, lua, model, meta, other
		const sorted = node.filePaths
			.slice() // Copy
			.filter((str) => allowBinaryFiles || !isBinaryFilePath(str))
			.map((str) => [str, getSortKey(str)] as [string, number])
			.sort((a, b) => a[1] - b[1])
			.map((pair) => pair[0]);
		const first = sorted.shift();
		return first ?? null;
	}
	return null;
};

export const getSourcemapNodeTreeOrder = (
	node: SourcemapNode,
	metadataProvider: MetadataProvider
): number | null => {
	let order = 0;

	const metadataOrder = metadataProvider.getExplorerOrder(node.className);
	if (metadataOrder) {
		order = metadataOrder;
	}

	// HACK: Always sort wally packages last
	if (node.wallyVersion) {
		order += 999_999;
	}

	return order;
};

export const cloneSourcemapNode = (
	node: SourcemapNode,
	deep: boolean | void
): SourcemapNode => {
	const filePaths = node.filePaths ? [...node.filePaths] : undefined;
	const children = node.children ? [...node.children] : undefined;
	if (deep && children) {
		for (const [index, child] of children.entries()) {
			children[index] = cloneSourcemapNode(child, true);
		}
	}
	return {
		name: node.name,
		className: node.className,
		folderPath: node.folderPath,
		filePaths,
		children,
	};
};

// NOTE: We reuse the same arrays here when file paths
// are missing to avoid allocation during the below
// path checks unless it is absolutely necessary
const EMPTY_PATHS_ARRAY: string[] = [];
const EMPTY_CHILDREN_ARRAY: SourcemapNode[] = [];
export const areSourcemapNodesEqual = (
	previous: SourcemapNode | undefined,
	current: SourcemapNode | undefined,
	deep: boolean | void
): boolean => {
	if (
		previous?.folderPath !== current?.folderPath ||
		previous?.className !== current?.className ||
		previous?.name !== current?.name ||
		previous?.wallyVersion !== current?.wallyVersion
	) {
		return false;
	}
	if ((previous && previous.filePaths) || (current && current.filePaths)) {
		const previousPaths = previous?.filePaths ?? EMPTY_PATHS_ARRAY;
		const currentPaths = current?.filePaths ?? EMPTY_PATHS_ARRAY;
		if (previousPaths.length !== currentPaths.length) {
			return false;
		} else if (previousPaths.length === 1) {
			if (previousPaths[0] !== currentPaths[0]) {
				return false;
			}
		} else {
			const previousSet = new Set(previousPaths);
			const currentSet = new Set(currentPaths);
			for (const current of currentSet) {
				if (!previousSet.has(current)) {
					return false;
				}
			}
			for (const previous of previousSet) {
				if (!currentSet.has(previous)) {
					return false;
				}
			}
		}
	}
	if (
		deep &&
		((previous && previous.children) || (current && current.children))
	) {
		const previousChildren = previous?.children ?? EMPTY_CHILDREN_ARRAY;
		const currentChildren = current?.children ?? EMPTY_CHILDREN_ARRAY;
		if (previousChildren.length !== currentChildren.length) {
			return false;
		} else if (previousChildren.length === 1) {
			if (
				!areSourcemapNodesEqual(previousChildren[0], currentChildren[0])
			) {
				return false;
			}
		} else {
			const indices = new Set([...previousChildren.keys()]);
			for (const index of currentChildren.keys()) {
				indices.add(index);
			}
			for (const index of indices.values()) {
				if (
					!areSourcemapNodesEqual(
						previousChildren[index],
						currentChildren[index]
					)
				) {
					return false;
				}
			}
		}
	}
	return true;
};

export const connectSourcemapUsingServer = (
	context: vscode.ExtensionContext,
	workspacePath: string,
	settings: SettingsProvider,
	treeProvider: RojoTreeProvider
): [() => boolean, () => void, () => void] => {
	// Create a callback for handling rpc messages
	let lastSourcemap: SourcemapNode | null = null;
	const callback = (_: any, message: RpcMessage) => {
		if (
			message.kind === "Notification" &&
			message.data.method === "InstanceDiff"
		) {
			const sourcemap: SourcemapNode | null =
				message.data.value?.data ?? null;
			if (sourcemap !== null) {
				postprocessSourcemap(workspacePath, settings, sourcemap);
				treeProvider.update(workspacePath, sourcemap);
			} else {
				treeProvider.delete(workspacePath);
			}
			lastSourcemap = sourcemap;
		}
	};

	// Start the server
	let childProcess = startServer(context, workspacePath, settings, callback);

	// Create callback for refreshing & reloading server
	const refresh = () => {
		if (lastSourcemap) {
			treeProvider.update(workspacePath, lastSourcemap, true);
			return true;
		} else {
			return false;
		}
	};
	const reload = async () => {
		childProcess.kill();
		childProcess = startServer(context, workspacePath, settings, callback);
	};

	// Create callback for disconnecting (destroying)
	// everything created for this workspace folder
	const destroy = () => {
		treeProvider.delete(workspacePath);
		childProcess.kill();
	};

	// Set as initially loading
	treeProvider.setLoading(workspacePath, undefined);

	return [refresh, reload, destroy];
};
