import * as path from "path";

const KNOWN_ICON_CLASSES: Set<string> = new Set([
	"Attachment",
	"BasePart",
	"BindableEvent",
	"BindableFunction",
	"DataModel",
	"Folder",
	"Instance",
	"Lighting",
	"LocalScript",
	"LuaSourceContainer",
	"MaterialService",
	"MaterialVariant",
	"MeshPart",
	"Model",
	"ModuleScript",
	"Part",
	"PlayerGui",
	"Players",
	"PlayerScripts",
	"RemoteEvent",
	"RemoteFunction",
	"ReplicatedFirst",
	"ReplicatedStorage",
	"RunService",
	"Script",
	"ServerScriptService",
	"ServerStorage",
	"SoundService",
	"StarterCharacterScripts",
	"StarterGui",
	"StarterPack",
	"StarterPlayer",
	"StarterPlayerScripts",
	"Teams",
	"Terrain",
	"TextChatService",
	"VoiceChatService",
	"Workspace",
	"WorldModel",
]);

export const getClassIconPath = (className: string): string => {
	if (KNOWN_ICON_CLASSES.has(className)) {
		return path.join(__dirname, "..", "..", "icons", `${className}.png`);
	} else {
		return path.join(__dirname, "..", "..", "icons", `Instance.png`);
	}
};
