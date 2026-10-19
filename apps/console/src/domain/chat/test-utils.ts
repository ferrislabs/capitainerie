export function sseEvent(event: string, data: unknown): string {
	return `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`
}

export function sseResponse(chunks: ReadonlyArray<string>): Response {
	const encoder = new TextEncoder()
	const body = new ReadableStream<Uint8Array>({
		start(controller) {
			for (const chunk of chunks) controller.enqueue(encoder.encode(chunk))
			controller.close()
		},
	})
	return new Response(body, {
		status: 200,
		headers: { 'Content-Type': 'text/event-stream' },
	})
}

export function openSse(signal?: AbortSignal) {
	const encoder = new TextEncoder()
	let controller!: ReadableStreamDefaultController<Uint8Array>
	const body = new ReadableStream<Uint8Array>({
		start(own) {
			controller = own
		},
	})
	signal?.addEventListener('abort', () => {
		try {
			controller.error(new DOMException('Aborted', 'AbortError'))
		} catch {
			return
		}
	})
	return {
		response: new Response(body, {
			status: 200,
			headers: { 'Content-Type': 'text/event-stream' },
		}),
		push: (chunk: string) => controller.enqueue(encoder.encode(chunk)),
		close: () => controller.close(),
	}
}
