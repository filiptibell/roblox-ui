import { downloadWithProgress } from "../axios";

const URL_REPO = "Elttob/Vanilla";
const URL_COMMIT = "7dd06bde94384b249055922f9818a87b3c3eba89";
const URL_BASE = `https://raw.githubusercontent.com/${URL_REPO}/${URL_COMMIT}`;

const PACK_PALETTES_URL = `${URL_BASE}/docs/icons/palettes.json`;
const PACK_ICON_DATA_URL = `${URL_BASE}/docs/icons/icondata.json`;
const PACK_ICONS_SVG_URL = `${URL_BASE}/docs/icons/icons.svg`;

type IconDataItem = {
	icon: number;
	name: string;
	glyph: string;
	colour: string;
};
type IconData = Array<IconDataItem>;

type PaletteId = "platinum" | "graphite" | "white";
type PaletteColor = "red" | "yellow" | "green" | "blue" | "purple" | "grey";
type PaletteItem = {
	id: PaletteId;
	title: string;
	colours: Record<PaletteColor, string>;
};
type Palettes = {
	defaults: {
		light: PaletteId;
		dark: PaletteId;
	};
	palettes: Array<PaletteItem>;
};

const DEFAULT_PALETTE: PaletteId = "platinum";

export const download = async (progressCallback: (progress: number) => any) => {
	let progressPalettes = 0;
	let progressIconData = 0;
	let progressIconsSvg = 0;
	const updateProgress = () => {
		progressCallback(
			(progressPalettes + progressIconData + progressIconsSvg) / 3
		);
	};

	const [palettes, iconData, iconsSvg]: [Palettes, IconData, string] =
		await Promise.all([
			downloadWithProgress(
				PACK_PALETTES_URL,
				(progress) => {
					progressPalettes = progress;
					updateProgress();
				},
				"json"
			),
			downloadWithProgress(
				PACK_ICON_DATA_URL,
				(progress) => {
					progressIconData = progress;
					updateProgress();
				},
				"json"
			),
			downloadWithProgress(PACK_ICONS_SVG_URL, (progress) => {
				progressIconsSvg = progress;
				updateProgress();
			}),
		]);

	// Parse icon data into map of instance name => index
	const instanceIndices = new Map<string, number>();
	for (const item of iconData.values()) {
		instanceIndices.set(item.name, item.icon);
	}

	// Extract svg lines and find out the range in which svg icon paths exist
	let svgLines = iconsSvg
		.trim()
		.split("\n")
		.map((line) => line.trim());
	let pathsFirstIndex = 0;
	let pathsLastIndex = svgLines.length - 1;
	for (let index = 0; index < pathsLastIndex; index++) {
		const line = svgLines[index];
		if (line.startsWith("<path")) {
			pathsFirstIndex = index;
			break;
		}
	}
	for (let index = pathsLastIndex; index > pathsFirstIndex; index--) {
		const line = svgLines[index];
		if (line.startsWith("<path")) {
			pathsLastIndex = index;
			break;
		}
	}

	// Extract each individual svg icon from iconsSvg set using index,
	// preserving the svg definition (everything before & after paths)
	const svgBefore = svgLines.slice(0, pathsFirstIndex - 1).join("\n");
	const svgAfter = svgLines.slice(pathsLastIndex + 1).join("\n");
	const svgIcons = new Map<number, string>();
	for (
		let pathIndex = pathsFirstIndex;
		pathIndex <= pathsLastIndex;
		pathIndex++
	) {
		const path = svgLines[pathIndex];
		const index = pathIndex - pathsFirstIndex + 1;
		const icon = `${svgBefore}\n${path}\n${svgAfter}`.replace(
			`viewBox="0 0 2160 16"`,
			`viewBox="${index * 16} 0 ${(index + 1) * 16} 16"`
		);
		svgIcons.set(index, icon);
	}

	// Gather color hex values to use for transforms
	const paletteHexColors = new Map<PaletteId, Record<PaletteColor, string>>();
	for (const palette of palettes.palettes) {
		paletteHexColors.set(palette.id, palette.colours);
	}
	const paletteColorsDefault = paletteHexColors.get(DEFAULT_PALETTE)!;
	const paletteColorsLight = paletteHexColors.get(palettes.defaults.light)!;
	const paletteColorsDark = paletteHexColors.get(palettes.defaults.dark)!;

	// Make light/dark variants of each individual icon using the transforms data
	const iconBuffersLight = new Map<number, Buffer>();
	const iconBuffersDark = new Map<number, Buffer>();
	for (const [index, svg] of svgIcons.entries()) {
		let light = svg;
		for (const key in paletteColorsDefault) {
			if (paletteColorsDefault.hasOwnProperty(key)) {
				const hex = paletteColorsDefault[key as PaletteColor];
				const repl = paletteColorsLight[key as PaletteColor];
				light = light.replace(hex, repl);
			}
		}
		let dark = svg;
		for (const key in paletteColorsDefault) {
			if (paletteColorsDefault.hasOwnProperty(key)) {
				const hex = paletteColorsDefault[key as PaletteColor];
				const repl = paletteColorsDark[key as PaletteColor];
				dark = dark.replace(hex, repl);
			}
		}
		iconBuffersLight.set(index, Buffer.from(light, "utf8"));
		iconBuffersDark.set(index, Buffer.from(dark, "utf8"));
	}

	// Insert into icons map as ClassName.svg => file string buffers
	const icons = new Map<string, { light: Buffer; dark: Buffer }>();
	for (const [name, index] of instanceIndices.entries()) {
		icons.set(`${name}.svg`, {
			light: iconBuffersLight.get(index)!,
			dark: iconBuffersDark.get(index)!,
		});
	}
	return icons;
};
