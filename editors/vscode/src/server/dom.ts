export type None = void | undefined | null;
export type Option<T> = None | T;

export type DomInstance = {
	id: string;
	className: string;
	name: string;
	children?: string[];
	metadata?: DomInstanceMetadata;
};

// Instance metadata

export type DomInstanceMetadata = {
	actions: DomInstanceMetadataActions;
	paths: DomInstanceMetadataPaths;
};

export type DomInstanceMetadataActions = {
	canOpen?: true;
	canMove?: true;
	canPasteSibling?: true;
	canPasteInto?: true;
	canInsertService?: true;
	canInsertObject?: true;
};

export type DomInstanceMetadataPaths = {
	folder?: string;
	file?: string;
	fileMeta?: string;
	rojo?: string;
	wally?: string;
	wallyLock?: string;
};

// Request & response types

export type DomRootRequest = None;
export type DomRootResponse = Option<DomInstance>;

export type DomGetRequest = { id: string };
export type DomGetResponse = Option<DomInstance>;

export type DomChildrenRequest = { id: string };
export type DomChildrenResponse = DomInstance[];

// Notifications

type DomNotificationAdded = {
	parentId?: string;
	childId: string;
};

type DomNotificationRemoved = {
	parentId?: string;
	childId: string;
};

type DomNotificationChanged = {
	id: string;
	className?: string;
	name?: string;
};

export type DomNotification =
	| null
	| { kind: "Added"; data: DomNotificationAdded }
	| { kind: "Removed"; data: DomNotificationRemoved }
	| { kind: "Changed"; data: DomNotificationChanged };

// Method -> request & response type maps

export type MethodRequestTypes = {
	["dom/notification"]: DomNotification;
	["dom/root"]: DomRootRequest;
	["dom/get"]: DomGetRequest;
	["dom/children"]: DomChildrenRequest;
};

export type MethodResponseTypes = {
	["dom/notification"]: None;
	["dom/root"]: DomRootResponse;
	["dom/get"]: DomGetResponse;
	["dom/children"]: DomChildrenResponse;
};
