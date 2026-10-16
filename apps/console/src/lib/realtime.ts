import type { Query, QueryClient } from '@tanstack/react-query'
import { Store } from '@tanstack/store'
import { authStore } from '#/store/auth.store'

export type RealtimeState = 'connected' | 'degraded' | 'offline'

export const realtimeStore = new Store<{ state: RealtimeState }>({
	state: 'offline',
})

export const REALTIME_PATH = '/api/v1/realtime'
export const BACKOFF_MIN_MS = 1000
export const BACKOFF_MAX_MS = 30000
export const SAFETY_REFETCH_MS = 60000

const SOCKET_OPEN = 1
const RUNS_ID = '/api/v1/runs'
const CARDS_ID = '/api/v1/cards'
const TIMELINE_ID = '/api/v1/runs/{run_id}/timeline'

export interface SocketLike {
	readyState: number
	send(data: string): void
	close(): void
	onopen: ((event: Event) => void) | null
	onmessage: ((event: MessageEvent) => void) | null
	onclose: ((event: CloseEvent) => void) | null
	onerror: ((event: Event) => void) | null
}

export type SocketFactory = (url: string) => SocketLike

export interface RealtimeOptions {
	url: string
	queryClient: QueryClient
	createSocket?: SocketFactory
}

interface EventMessage {
	op: 'event'
	kind: string
	run_id?: string
	card_id?: string
}

type ServerMessage =
	| { op: 'ready' }
	| { op: 'degraded' }
	| { op: 'resync' }
	| EventMessage

export function realtimeUrl(base: string): string {
	const url = new URL(REALTIME_PATH, base)
	url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
	return url.toString()
}

export function setRealtimeState(state: RealtimeState): void {
	if (realtimeStore.state.state !== state) {
		realtimeStore.setState(() => ({ state }))
	}
}

export function parseServerMessage(raw: unknown): ServerMessage | undefined {
	if (typeof raw !== 'string') return undefined
	try {
		const value: unknown = JSON.parse(raw)
		if (typeof value !== 'object' || value === null || !('op' in value)) {
			return undefined
		}
		const op = (value as { op: unknown }).op
		if (op === 'ready' || op === 'degraded' || op === 'resync') {
			return value as ServerMessage
		}
		if (op === 'event' && typeof (value as EventMessage).kind === 'string') {
			return value as EventMessage
		}
	} catch {
		return undefined
	}
	return undefined
}

function queryKeyHead(query: Query): {
	_id?: string
	path?: { run_id?: string }
} {
	const head: unknown = query.queryKey[0]
	return typeof head === 'object' && head !== null ? head : {}
}

export function invalidateForEvent(
	queryClient: QueryClient,
	event: EventMessage,
): void {
	const { kind, run_id: runId } = event
	if (kind.startsWith('run.')) {
		void queryClient.invalidateQueries({
			predicate: (query) => {
				const { _id: id, path } = queryKeyHead(query)
				if (id === RUNS_ID) return true
				if (!id?.startsWith(`${RUNS_ID}/`)) return false
				return runId === undefined || path?.run_id === runId
			},
		})
		return
	}
	if (kind.startsWith('card.')) {
		void queryClient.invalidateQueries({
			predicate: (query) => {
				const { _id: id, path } = queryKeyHead(query)
				if (id?.startsWith(CARDS_ID)) return true
				return (
					runId !== undefined && id === TIMELINE_ID && path?.run_id === runId
				)
			},
		})
	}
}

export function nextBackoff(current: number): number {
	return Math.min(current * 2, BACKOFF_MAX_MS)
}

export function startRealtime(options: RealtimeOptions): () => void {
	const { url, queryClient } = options
	const createSocket: SocketFactory =
		options.createSocket ?? ((target) => new WebSocket(target))

	let socket: SocketLike | null = null
	let identifiedToken: string | null = null
	let backoff = BACKOFF_MIN_MS
	let timer: ReturnType<typeof setTimeout> | null = null
	let stopped = false

	function identify(target: SocketLike, token: string) {
		identifiedToken = token
		target.send(JSON.stringify({ op: 'identify', token }))
	}

	function clearTimer() {
		if (timer !== null) {
			clearTimeout(timer)
			timer = null
		}
	}

	function schedule() {
		if (stopped || timer !== null || !authStore.state.accessToken) return
		timer = setTimeout(() => {
			timer = null
			connect()
		}, backoff)
		backoff = nextBackoff(backoff)
	}

	function connect() {
		if (stopped || socket || !authStore.state.accessToken) return
		const current = createSocket(url)
		socket = current
		current.onopen = () => {
			const latest = authStore.state.accessToken
			if (latest) identify(current, latest)
		}
		current.onmessage = (event) => {
			const message = parseServerMessage(event.data)
			if (!message) return
			switch (message.op) {
				case 'ready':
					backoff = BACKOFF_MIN_MS
					setRealtimeState('connected')
					break
				case 'degraded':
					setRealtimeState('degraded')
					break
				case 'resync':
					void queryClient.invalidateQueries()
					break
				case 'event':
					invalidateForEvent(queryClient, message)
					break
			}
		}
		current.onclose = () => {
			if (socket !== current) return
			socket = null
			identifiedToken = null
			setRealtimeState('offline')
			schedule()
		}
		current.onerror = () => {}
	}

	const subscription = authStore.subscribe(() => {
		const token = authStore.state.accessToken
		if (!token) {
			clearTimer()
			socket?.close()
			return
		}
		if (!socket) {
			clearTimer()
			backoff = BACKOFF_MIN_MS
			connect()
			return
		}
		if (socket.readyState === SOCKET_OPEN && token !== identifiedToken) {
			identify(socket, token)
		}
	})

	connect()

	return () => {
		stopped = true
		subscription.unsubscribe()
		clearTimer()
		const closing = socket
		socket = null
		closing?.close()
		setRealtimeState('offline')
	}
}
