import { describe, expect, it } from 'vitest'
import { SseDecoder } from '#/domain/chat/sse'

describe('SseDecoder', () => {
	it('ends a message at the blank line, whatever the chunking', () => {
		const decoder = new SseDecoder()

		expect(decoder.push('event: delta\ndata: {"te')).toEqual([])
		expect(decoder.push('xt":"a"}\n\nevent: done\ndata: {}\n\n')).toEqual([
			{ event: 'delta', data: '{"text":"a"}' },
			{ event: 'done', data: '{}' },
		])
	})

	it('reads CRLF line ends, joins data lines and skips comments', () => {
		const decoder = new SseDecoder()

		expect(
			decoder.push(
				': keep-alive\r\nevent: x\r\ndata: one\r\ndata: two\r\n\r\n',
			),
		).toEqual([{ event: 'x', data: 'one\ntwo' }])
	})

	it('names a message without event "message" and forgets the event after it', () => {
		const decoder = new SseDecoder()

		expect(decoder.push('data: a\n\nevent: b\ndata: c\n\ndata: d\n\n')).toEqual(
			[
				{ event: 'message', data: 'a' },
				{ event: 'b', data: 'c' },
				{ event: 'message', data: 'd' },
			],
		)
	})

	it('flushes a last message that lacks its blank line', () => {
		const decoder = new SseDecoder()
		decoder.push('event: done\ndata: {}\n')

		expect(decoder.flush()).toEqual([{ event: 'done', data: '{}' }])
		expect(decoder.flush()).toEqual([])
	})

	it('ignores a blank line with no data', () => {
		const decoder = new SseDecoder()

		expect(decoder.push('event: lonely\n\n')).toEqual([])
	})
})
