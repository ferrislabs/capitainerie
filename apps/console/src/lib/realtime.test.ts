import { type InvalidateQueryFilters, QueryClient } from '@tanstack/react-query'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
	BACKOFF_MAX_MS,
	BACKOFF_MIN_MS,
	nextBackoff,
	parseServerMessage,
	realtimeStore,
	realtimeUrl,
	type SocketLike,
	startRealtime,
} from '#/lib/realtime'
import { clearAuth, setAuth } from '#/store/auth.store'

class MockSocket implements SocketLike {
	static instances: MockSocket[] = []
	readyState = 0
	sent: string[] = []
	closed = false
	onopen: SocketLike['onopen'] = null
	onmessage: SocketLike['onmessage'] = null
	onclose: SocketLike['onclose'] = null
	onerror: SocketLike['onerror'] = null

	constructor(readonly url: string) {
		MockSocket.instances.push(this)
	}

	send(data: string) {
		this.sent.push(data)
	}

	close() {
		this.closed = true
		this.readyState = 3
		this.onclose?.(new CloseEvent('close'))
	}

	open() {
		this.readyState = 1
		this.onopen?.(new Event('open'))
	}

	receive(message: unknown) {
		this.onmessage?.(
			new MessageEvent('message', { data: JSON.stringify(message) }),
		)
	}

	drop() {
		this.readyState = 3
		this.onclose?.(new CloseEvent('close'))
	}
}

const URL = 'ws://api.test/api/v1/realtime'

function key(id: string, path?: Record<string, string>) {
	return [{ _id: id, ...(path ? { path } : {}) }]
}

describe('realtime helpers', () => {
	it('derives a ws url from the api base', () => {
		expect(realtimeUrl('http://localhost:4100')).toBe(
			'ws://localhost:4100/api/v1/realtime',
		)
		expect(realtimeUrl('https://api.example.com')).toBe(
			'wss://api.example.com/api/v1/realtime',
		)
	})

	it('doubles the backoff up to the cap', () => {
		expect(nextBackoff(1000)).toBe(2000)
		expect(nextBackoff(20000)).toBe(BACKOFF_MAX_MS)
	})

	it('ignores malformed messages', () => {
		expect(parseServerMessage('nope')).toBeUndefined()
		expect(parseServerMessage('{"op":"wat"}')).toBeUndefined()
		expect(parseServerMessage(42)).toBeUndefined()
		expect(parseServerMessage('{"op":"ready"}')).toEqual({ op: 'ready' })
	})
})

describe('startRealtime', () => {
	let client: QueryClient
	let filters: Array<InvalidateQueryFilters | undefined>
	let stop: () => void

	function start() {
		stop = startRealtime({
			url: URL,
			queryClient: client,
			createSocket: (url) => new MockSocket(url),
		})
	}

	function latest() {
		return MockSocket.instances[MockSocket.instances.length - 1]
	}

	function invalidated(queryKey: unknown[]) {
		const predicates = filters.map((filter) => filter?.predicate)
		const query = { queryKey } as never
		return predicates.some((predicate) => predicate?.(query))
	}

	beforeEach(() => {
		vi.useFakeTimers()
		MockSocket.instances = []
		client = new QueryClient()
		filters = []
		vi.spyOn(client, 'invalidateQueries').mockImplementation(async (filter) => {
			filters.push(filter)
		})
		setAuth({ accessToken: 'tok-1', isAuthenticated: true })
		realtimeStore.setState(() => ({ state: 'offline' }))
	})

	afterEach(() => {
		stop?.()
		clearAuth()
		vi.useRealTimers()
	})

	it('sends identify first, with the current token, then becomes connected', () => {
		start()
		const socket = latest()
		expect(socket.url).toBe(URL)
		expect(socket.sent).toEqual([])
		socket.open()
		expect(socket.sent).toEqual([
			JSON.stringify({ op: 'identify', token: 'tok-1' }),
		])
		expect(realtimeStore.state.state).toBe('offline')
		socket.receive({ op: 'ready' })
		expect(realtimeStore.state.state).toBe('connected')
	})

	it('does not connect without a token', () => {
		clearAuth()
		start()
		expect(MockSocket.instances).toHaveLength(0)
	})

	it('re-identifies when the token changes', () => {
		start()
		const socket = latest()
		socket.open()
		socket.receive({ op: 'ready' })
		setAuth({ accessToken: 'tok-2' })
		expect(socket.sent).toHaveLength(2)
		expect(JSON.parse(socket.sent[1])).toEqual({
			op: 'identify',
			token: 'tok-2',
		})
		expect(MockSocket.instances).toHaveLength(1)
	})

	it('reports the degraded mode', () => {
		start()
		const socket = latest()
		socket.open()
		socket.receive({ op: 'ready' })
		socket.receive({ op: 'degraded' })
		expect(realtimeStore.state.state).toBe('degraded')
	})

	it('invalidates the runs list and the run queries on run events', () => {
		start()
		latest().receive({ op: 'event', kind: 'run.started', run_id: 'r1' })
		expect(invalidated(key('/api/v1/runs'))).toBe(true)
		expect(
			invalidated(key('/api/v1/runs/{run_id}/timeline', { run_id: 'r1' })),
		).toBe(true)
		expect(
			invalidated(key('/api/v1/runs/{run_id}/timeline', { run_id: 'r2' })),
		).toBe(false)
		expect(invalidated(key('/api/v1/cards'))).toBe(false)
	})

	it('invalidates the cards list, and the timeline when a run is carried', () => {
		start()
		latest().receive({
			op: 'event',
			kind: 'card.moved',
			card_id: 'c1',
			run_id: 'r1',
		})
		expect(invalidated(key('/api/v1/cards'))).toBe(true)
		expect(
			invalidated(key('/api/v1/runs/{run_id}/timeline', { run_id: 'r1' })),
		).toBe(true)
		expect(invalidated(key('/api/v1/runs'))).toBe(false)
	})

	it('reloads the open card panel when a comment is added', () => {
		start()
		latest().receive({
			op: 'event',
			kind: 'card.commented',
			card_id: 'c1',
		})
		expect(invalidated(key('/api/v1/cards/{card_id}', { card_id: 'c1' }))).toBe(
			true,
		)
	})

	it('does not touch the timeline for a card event without run', () => {
		start()
		latest().receive({ op: 'event', kind: 'card.updated', card_id: 'c1' })
		expect(
			invalidated(key('/api/v1/runs/{run_id}/timeline', { run_id: 'r1' })),
		).toBe(false)
	})

	it('invalidates everything on resync', () => {
		start()
		latest().receive({ op: 'resync' })
		expect(filters).toEqual([undefined])
	})

	it('reconnects with an exponential backoff, reset once ready', () => {
		start()
		latest().drop()
		expect(realtimeStore.state.state).toBe('offline')
		expect(MockSocket.instances).toHaveLength(1)

		vi.advanceTimersByTime(BACKOFF_MIN_MS - 1)
		expect(MockSocket.instances).toHaveLength(1)
		vi.advanceTimersByTime(1)
		expect(MockSocket.instances).toHaveLength(2)

		latest().drop()
		vi.advanceTimersByTime(BACKOFF_MIN_MS * 2 - 1)
		expect(MockSocket.instances).toHaveLength(2)
		vi.advanceTimersByTime(1)
		expect(MockSocket.instances).toHaveLength(3)

		latest().open()
		latest().receive({ op: 'ready' })
		latest().drop()
		vi.advanceTimersByTime(BACKOFF_MIN_MS)
		expect(MockSocket.instances).toHaveLength(4)
	})

	it('caps the backoff at 30 s', () => {
		start()
		for (let i = 0; i < 8; i++) {
			latest().drop()
			vi.advanceTimersByTime(BACKOFF_MAX_MS)
		}
		const before = MockSocket.instances.length
		latest().drop()
		vi.advanceTimersByTime(BACKOFF_MAX_MS)
		expect(MockSocket.instances).toHaveLength(before + 1)
	})

	it('closes and stops reconnecting once stopped', () => {
		start()
		const socket = latest()
		stop()
		expect(socket.closed).toBe(true)
		vi.advanceTimersByTime(BACKOFF_MAX_MS * 2)
		expect(MockSocket.instances).toHaveLength(1)
	})

	it('closes when the user signs out and reconnects on sign in', () => {
		start()
		const socket = latest()
		clearAuth()
		expect(socket.closed).toBe(true)
		vi.advanceTimersByTime(BACKOFF_MAX_MS)
		expect(MockSocket.instances).toHaveLength(1)
		setAuth({ accessToken: 'tok-3', isAuthenticated: true })
		expect(MockSocket.instances).toHaveLength(2)
	})
})
