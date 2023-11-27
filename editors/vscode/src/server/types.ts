// biome-ignore lint/suspicious/noConfusingVoidType: let handlers and such return nothing
export type None = undefined | null | void;
export type Option<T> = None | T;

export type DomInstance = {
	id: string;
	parentId?: string;
	className: string;
	name: string;
	children?: string[];
	metadata?: DomInstanceMetadata;
};

// Instance metadata

export type DomInstanceMetadata = {
	package?: DomInstanceMetadataPackage;
	actions?: DomInstanceMetadataActions;
	paths?: DomInstanceMetadataPaths;
};

export type DomInstanceMetadataPackage = {
	scope: string;
	name: string;
	version: string;
	isRoot: boolean;
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

export type DomAncestorsRequest = { id: string };
export type DomAncestorsResponse = DomInstance[];

export type DomFindByPathRequest = { path: string };
export type DomFindByPathResponse = Option<DomInstance>;

export type DomFindByQueryRequest = { query: string; limit: Option<number> };
export type DomFindByQueryResponse = DomInstance[];

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

export type MethodTypes = {
	"dom/notification": {
		request: DomNotification;
		response: None;
	};
	"dom/root": {
		request: DomRootRequest;
		response: DomRootResponse;
	};
	"dom/get": {
		request: DomGetRequest;
		response: DomGetResponse;
	};
	"dom/children": {
		request: DomChildrenRequest;
		response: DomChildrenResponse;
	};
	"dom/ancestors": {
		request: DomAncestorsRequest;
		response: DomAncestorsResponse;
	};
	"dom/findByPath": {
		request: DomFindByPathRequest;
		response: DomFindByPathResponse;
	};
	"dom/findByQuery": {
		request: DomFindByQueryRequest;
		response: DomFindByQueryResponse;
	};
};
