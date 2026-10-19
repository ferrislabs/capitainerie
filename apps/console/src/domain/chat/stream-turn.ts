import { api } from '#/api'
import { authHeaders, toApiError } from '#/api/api.fetch'
import { readTurnEvents, type TurnEvent } from '#/domain/chat/turn-events'

export function messagesPath(conversationId: string): string {
	return `/api/v1/conversations/${encodeURIComponent(conversationId)}/messages`
}

export async function streamTurn(
	conversationId: string,
	message: string | undefined,
	signal: AbortSignal,
): Promise<AsyncGenerator<TurnEvent>> {
	const headers = authHeaders()
	headers.set('Content-Type', 'application/json')
	headers.set('Accept', 'text/event-stream')
	const response = await fetch(
		`${api.baseUrl}${messagesPath(conversationId)}`,
		{
			method: 'POST',
			headers,
			body: JSON.stringify(message === undefined ? {} : { message }),
			signal,
		},
	)
	if (!response.ok) throw await toApiError(response)
	if (response.body === null) {
		throw new Error('La réponse du Capitaine est vide.')
	}
	return readTurnEvents(response.body)
}
