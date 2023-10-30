import { downloadWithProgress } from "../axios";
import { RobloxApiDump, RobloxReflectionMetadata } from "../roblox";

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

export const download = async (
	apiDump: RobloxApiDump,
	reflection: RobloxReflectionMetadata,
	progressCallback: (progress: number) => any
) => {
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
	let highestKnownIndexInPack = 0;
	for (const item of iconData.values()) {
		instanceIndices.set(item.name, item.icon);
		if (item.icon > highestKnownIndexInPack) {
			highestKnownIndexInPack = item.icon;
		}
	}
	for (const [className, item] of reflection.Classes.entries()) {
		if (
			item.ExplorerImageIndex &&
			item.ExplorerImageIndex <= highestKnownIndexInPack &&
			!instanceIndices.has(className)
		) {
			instanceIndices.set(className, item.ExplorerImageIndex);
		}
	}

	// Gather color hex values to use for transforms
	const paletteHexColors = new Map<PaletteId, Record<PaletteColor, string>>();
	for (const palette of palettes.palettes) {
		paletteHexColors.set(palette.id, palette.colours);
	}
	const paletteColorsDefault = paletteHexColors.get(palettes.defaults.light)!;
	const paletteColorsLight = paletteHexColors.get(palettes.defaults.light)!;
	const paletteColorsDark = paletteHexColors.get(palettes.defaults.dark)!;

	// Make light & dark variants of svg
	const iconsSvgFixed = iconsSvg.replace(
		new RegExp("006FB3", "gi"),
		"006FB2"
	);
	let iconsSvgLight = iconsSvgFixed;
	for (const key in paletteColorsDefault) {
		if (paletteColorsDefault.hasOwnProperty(key)) {
			const hex = paletteColorsDefault[key as PaletteColor];
			const repl = paletteColorsLight[key as PaletteColor];
			iconsSvgLight = iconsSvgLight.replace(new RegExp(hex, "gi"), repl);
		}
	}
	let iconsSvgDark = iconsSvgFixed;
	for (const key in paletteColorsDefault) {
		if (paletteColorsDefault.hasOwnProperty(key)) {
			const hex = paletteColorsDefault[key as PaletteColor];
			const repl = paletteColorsDark[key as PaletteColor];
			iconsSvgDark = iconsSvgDark.replace(new RegExp(hex, "gi"), repl);
		}
	}

	// Make light/dark variants of each individual icon using the transforms data
	const iconBuffersLight = new Map<number, Buffer>();
	const iconBuffersDark = new Map<number, Buffer>();
	for (const [name, index] of instanceIndices.entries()) {
		const light = iconsSvgLight
			.replace(`width="2160"`, `width="16"`)
			.replace(
				`viewBox="0 0 2160 16"`,
				`viewBox="${index * 16} 0 16 16"`
			);
		const dark = iconsSvgDark
			.replace(`width="2160"`, `width="16"`)
			.replace(
				`viewBox="0 0 2160 16"`,
				`viewBox="${index * 16} 0 16 16"`
			);
		iconBuffersLight.set(index, Buffer.from(light, "utf8"));
		iconBuffersDark.set(index, Buffer.from(dark, "utf8"));
	}

	// Insert into icons map as ClassName.svg => file string buffers
	const icons = new Map<string, { light: Buffer; dark: Buffer }>();
	for (const [name, index] of instanceIndices.entries()) {
		const light = iconBuffersLight.get(index);
		const dark = iconBuffersDark.get(index);
		if (light && dark) {
			icons.set(`${name}.svg`, { light, dark });
			if (index === 0) {
				icons.set("Instance.svg", { light, dark });
			}
		}
	}
	return icons;
};
