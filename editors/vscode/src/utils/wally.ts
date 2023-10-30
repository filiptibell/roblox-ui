import * as vscode from "vscode";
import * as path from "path";
import { SourcemapNode } from "./sourcemap";

const fs = vscode.workspace.fs;

const REQUIRE_REGEX =
	/require\(script\.Parent\._Index\["([^"]+)"\]\["([^"]+)"\]\)/;
const WALLY_SPEC_REGEX = /([-\w]+)_([-\w]+)@([\d.]+)/;

export type PackageSource = {
	originalName: string;
	outerName: string;
	innerName: string;
};

export type WallySpec = {
	scope: string;
	name: string;
	version: string;
};

export const findPackageSource = async (
	originalName: string,
	packageLinkPath: string
): Promise<PackageSource | undefined> => {
	if (typeof packageLinkPath !== "string" || packageLinkPath.length <= 0) {
		throw new Error(
			"Package link path must be a non-empty string, got " +
				typeof packageLinkPath
		);
	}

	const packageLinkUri = vscode.Uri.file(packageLinkPath);

	const packageLinkSource = await fs
		.readFile(packageLinkUri)
		.then((bytes) => bytes.toString());

	const match = REQUIRE_REGEX.exec(packageLinkSource);

	if (match) {
		return {
			originalName,
			outerName: match[1],
			innerName: match[2],
		};
	} else {
		return undefined;
	}
};

export const findPackageSourceNode = (
	wallyIndexNode: SourcemapNode,
	source: PackageSource
): SourcemapNode | undefined => {
	if (!wallyIndexNode.children) {
		return undefined;
	}

	let foundOuter: SourcemapNode | undefined;
	for (const child of wallyIndexNode.children.values()) {
		if (child.name === source.outerName) {
			foundOuter = child;
			break;
		}
	}
	if (!foundOuter || !foundOuter.children) {
		return undefined;
	}

	let foundInner: SourcemapNode | undefined;
	for (const child of foundOuter.children.values()) {
		if (child.name === source.innerName) {
			foundInner = child;
			break;
		}
	}
	return foundInner;
};

export const parseWallySpec = (specStr: string): WallySpec | undefined => {
	const match = WALLY_SPEC_REGEX.exec(specStr);

	if (match) {
		return {
			scope: match[1],
			name: match[2],
			version: match[3],
		};
	} else {
		return undefined;
	}
};
