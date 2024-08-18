export type RpcMessageKind = "Request" | "Response"

export type RpcMessageData = {
	id: number
	method: string
	value?: unknown
}

export type RpcMessage = {
	kind: RpcMessageKind
	data: RpcMessageData
}

// biome-ignore lint/suspicious/noExplicitAny:
export const isRpcMessage = (arg: any): arg is RpcMessage => {
	return (
		typeof arg === "object" &&
		(arg.kind === "Request" || arg.kind === "Response") &&
		typeof arg.data === "object" &&
		typeof arg.data.id === "number" &&
		typeof arg.data.method === "string"
	)
}

export const createRpcRequest = (method: string, id: number, request?: unknown): RpcMessage => {
	return {
		kind: "Request",
		data: {
			id,
			method,
			value: request,
		},
	}
}

export const respondToRpcMessage = (message: RpcMessage, response?: unknown): RpcMessage => {
	return {
		kind: "Response",
		data: {
			id: message.data.id,
			method: message.data.method,
			value: response,
		},
	}
}
