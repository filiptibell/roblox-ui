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

export const download = async () => {
	const [palettes, iconData, iconsSvg]: [Palettes, IconData, string] =
		await Promise.all([
			downloadWithProgress(PACK_PALETTES_URL, () => {}, "json"),
			downloadWithProgress(PACK_ICON_DATA_URL, () => {}, "json"),
			downloadWithProgress(PACK_ICONS_SVG_URL, () => {}),
		]);

	// TODO: Parse icon data into map of instance name => index
	// TODO: Extract each individual svg icon from iconsSvg set using index
	// TODO: Make light/dark variants of each individual icon using the palettes data
	// TODO: Insert into icons map as ClassName.vsg => file string buffers

	const icons = new Map<string, { light: Buffer; dark: Buffer }>();
	return icons;
};
