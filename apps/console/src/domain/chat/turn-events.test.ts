import { describe, expect, it } from 'vitest'
import { sseEvent } from '#/domain/chat/test-utils'
import { parseTurnEvent, readTurnEvents } from '#/domain/chat/turn-events'

async function collect(stream: ReadableStream<Uint8Array>) {
	const events = []
	for await (const event of readTurnEvents(stream)) events.push(event)
	return events
}

function streamOf(bytes: Uint8Array, size: number) {
	return new ReadableStream<Uint8Array>({
		start(controller) {
			for (let at = 0; at < bytes.length; at += size) {
				controller.enqueue(bytes.slice(at, at + size))
			}
			controller.close()
		},
	})
}

describe('parseTurnEvent', () => {
	it('reads the five event kinds', () => {
		expect(parseTurnEvent({ event: 'delta', data: '{"text":"Bon"}' })).toEqual({
			type: 'delta',
			text: 'Bon',
		})
		expect(
			parseTurnEvent({
				event: 'tool',
				data: '{"phase":"start","id":"t1","name":"search_cards","arguments":"{}"}',
			}),
		).toEqual({
			type: 'tool-start',
			id: 't1',
			name: 'search_cards',
			arguments: '{}',
		})
		expect(
			parseTurnEvent({
				event: 'tool',
				data: '{"phase":"end","id":"t1","name":"search_cards","succeeded":true,"latency_ms":12,"summary":"aucune carte"}',
			}),
		).toEqual({
			type: 'tool-end',
			id: 't1',
			name: 'search_cards',
			succeeded: true,
			latencyMs: 12,
			summary: 'aucune carte',
		})
		expect(
			parseTurnEvent({ event: 'card', data: '{"card_id":"c1","title":"T"}' }),
		).toEqual({ type: 'card', cardId: 'c1', title: 'T' })
		expect(
			parseTurnEvent({
				event: 'done',
				data: '{"status":"complete","usage":{"input_tokens":3,"output_tokens":4,"cost_micros":5}}',
			}),
		).toEqual({
			type: 'done',
			status: 'complete',
			usage: { inputTokens: 3, outputTokens: 4, costMicros: 5 },
		})
		expect(
			parseTurnEvent({
				event: 'error',
				data: '{"code":"provider_error","message":"quota"}',
			}),
		).toEqual({ type: 'error', code: 'provider_error', message: 'quota' })
	})

	it('drops unknown, malformed and incomplete events', () => {
		expect(parseTurnEvent({ event: 'ping', data: '{}' })).toBeUndefined()
		expect(parseTurnEvent({ event: 'delta', data: 'not json' })).toBeUndefined()
		expect(parseTurnEvent({ event: 'delta', data: '[]' })).toBeUndefined()
		expect(parseTurnEvent({ event: 'delta', data: '{}' })).toBeUndefined()
		expect(
			parseTurnEvent({
				event: 'tool',
				data: '{"phase":"pause","id":"a","name":"b"}',
			}),
		).toBeUndefined()
		expect(
			parseTurnEvent({ event: 'card', data: '{"card_id":"c1"}' }),
		).toBeUndefined()
	})
})

describe('readTurnEvents', () => {
	const text =
		sseEvent('delta', { text: 'Je cherche é' }) +
		sseEvent('card', { card_id: 'c1', title: 'Ajouter un README' }) +
		sseEvent('done', {
			status: 'complete',
			usage: { input_tokens: 1, output_tokens: 2 },
		})

	it('yields the events in order from a byte stream cut anywhere', async () => {
		const bytes = new TextEncoder().encode(text)

		for (const size of [1, 3, 7, bytes.length]) {
			const events = await collect(streamOf(bytes, size))

			expect(events.map((event) => event.type)).toEqual([
				'delta',
				'card',
				'done',
			])
			expect(events[0]).toEqual({ type: 'delta', text: 'Je cherche é' })
		}
	})

	it('yields a final event that lacks its blank line', async () => {
		const bytes = new TextEncoder().encode(
			'event: delta\ndata: {"text":"a"}\n\nevent: done\ndata: {"status":"complete"}\n',
		)

		const events = await collect(streamOf(bytes, 5))

		expect(events.map((event) => event.type)).toEqual(['delta', 'done'])
	})
})
