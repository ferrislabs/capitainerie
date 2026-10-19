export interface SseMessage {
	event: string
	data: string
}

export class SseDecoder {
	private pending = ''
	private event = ''
	private data: Array<string> = []

	push(chunk: string): Array<SseMessage> {
		this.pending += chunk
		const messages: Array<SseMessage> = []
		let newline = this.pending.indexOf('\n')
		while (newline !== -1) {
			const line = this.pending.slice(0, newline).replace(/\r$/, '')
			this.pending = this.pending.slice(newline + 1)
			const message = this.read(line)
			if (message) messages.push(message)
			newline = this.pending.indexOf('\n')
		}
		return messages
	}

	flush(): Array<SseMessage> {
		const message = this.dispatch()
		return message ? [message] : []
	}

	private read(line: string): SseMessage | undefined {
		if (line === '') return this.dispatch()
		if (line.startsWith(':')) return undefined
		const colon = line.indexOf(':')
		const field = colon === -1 ? line : line.slice(0, colon)
		let value = colon === -1 ? '' : line.slice(colon + 1)
		if (value.startsWith(' ')) value = value.slice(1)
		if (field === 'event') this.event = value
		if (field === 'data') this.data.push(value)
		return undefined
	}

	private dispatch(): SseMessage | undefined {
		const event = this.event || 'message'
		const data = this.data
		this.event = ''
		this.data = []
		return data.length === 0 ? undefined : { event, data: data.join('\n') }
	}
}
