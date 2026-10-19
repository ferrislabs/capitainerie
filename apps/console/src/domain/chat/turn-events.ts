import { SseDecoder, type SseMessage } from '#/domain/chat/sse'

export interface TurnUsage {
	inputTokens: number
	outputTokens: number
	costMicros: number
}

export type TurnEvent =
	| { type: 'delta'; text: string }
	| {
			type: 'tool-start'
			id: string
			name: string
			arguments: string
	  }
	| {
			type: 'tool-end'
			id: string
			name: string
			succeeded: boolean
			latencyMs: number
			summary: string
	  }
	| { type: 'card'; cardId: string; title: string }
	| { type: 'done'; status: string; usage: TurnUsage }
	| { type: 'error'; code: string; message: string }

type Payload = Record<string, unknown>

function isPayload(value: unknown): value is Payload {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function text(payload: Payload, key: string): string | undefined {
	const value = payload[key]
	return typeof value === 'string' ? value : undefined
}

function count(payload: Payload, key: string): number {
	const value = payload[key]
	return typeof value === 'number' ? value : 0
}

function parseTool(payload: Payload): TurnEvent | undefined {
	const id = text(payload, 'id')
	const name = text(payload, 'name')
	if (id === undefined || name === undefined) return undefined
	if (payload.phase === 'start') {
		return {
			type: 'tool-start',
			id,
			name,
			arguments: text(payload, 'arguments') ?? '',
		}
	}
	if (payload.phase === 'end') {
		return {
			type: 'tool-end',
			id,
			name,
			succeeded: payload.succeeded === true,
			latencyMs: count(payload, 'latency_ms'),
			summary: text(payload, 'summary') ?? '',
		}
	}
	return undefined
}

function parseDone(payload: Payload): TurnEvent {
	const usage = isPayload(payload.usage) ? payload.usage : {}
	return {
		type: 'done',
		status: text(payload, 'status') ?? 'complete',
		usage: {
			inputTokens: count(usage, 'input_tokens'),
			outputTokens: count(usage, 'output_tokens'),
			costMicros: count(usage, 'cost_micros'),
		},
	}
}

export function parseTurnEvent(message: SseMessage): TurnEvent | undefined {
	let payload: unknown
	try {
		payload = JSON.parse(message.data)
	} catch {
		return undefined
	}
	if (!isPayload(payload)) return undefined
	switch (message.event) {
		case 'delta': {
			const delta = text(payload, 'text')
			return delta === undefined ? undefined : { type: 'delta', text: delta }
		}
		case 'tool':
			return parseTool(payload)
		case 'card': {
			const cardId = text(payload, 'card_id')
			const title = text(payload, 'title')
			return cardId === undefined || title === undefined
				? undefined
				: { type: 'card', cardId, title }
		}
		case 'done':
			return parseDone(payload)
		case 'error':
			return {
				type: 'error',
				code: text(payload, 'code') ?? 'error',
				message: text(payload, 'message') ?? '',
			}
		default:
			return undefined
	}
}

export async function* readTurnEvents(
	body: ReadableStream<Uint8Array>,
): AsyncGenerator<TurnEvent> {
	const reader = body.getReader()
	const decoder = new TextDecoder()
	const sse = new SseDecoder()
	try {
		while (true) {
			const { done, value } = await reader.read()
			if (done) break
			for (const message of sse.push(decoder.decode(value, { stream: true }))) {
				const event = parseTurnEvent(message)
				if (event) yield event
			}
		}
		for (const message of sse.push(decoder.decode())) {
			const event = parseTurnEvent(message)
			if (event) yield event
		}
		for (const message of sse.flush()) {
			const event = parseTurnEvent(message)
			if (event) yield event
		}
	} finally {
		reader.releaseLock()
	}
}
