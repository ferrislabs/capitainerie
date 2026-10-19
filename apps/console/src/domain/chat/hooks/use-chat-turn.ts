import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useRef, useState } from 'react'
import { chatErrorMessage, streamErrorMessage } from '#/domain/chat/chat-errors'
import {
	applyTurnEvent,
	type Draft,
	EMPTY_DRAFT,
} from '#/domain/chat/chat-state'
import { isConversationQuery } from '#/domain/chat/hooks/conversation-queries'
import { streamTurn } from '#/domain/chat/stream-turn'

export type TurnStatus = 'idle' | 'streaming'

export interface TurnState {
	status: TurnStatus
	pendingMessage?: string
	draft: Draft
	error?: string
}

const IDLE: TurnState = { status: 'idle', draft: EMPTY_DRAFT }

export function useChatTurn(conversationId: string) {
	const queryClient = useQueryClient()
	const controller = useRef<AbortController | null>(null)
	const [state, setState] = useState<TurnState>(IDLE)

	useEffect(() => () => controller.current?.abort(), [])

	const refresh = useCallback(
		() =>
			queryClient.invalidateQueries({
				predicate: (query) => isConversationQuery(query.queryKey),
			}),
		[queryClient],
	)

	const send = useCallback(
		async (message?: string) => {
			controller.current?.abort()
			const own = new AbortController()
			controller.current = own
			setState({
				status: 'streaming',
				pendingMessage: message,
				draft: EMPTY_DRAFT,
			})
			let failure: string | undefined
			try {
				const events = await streamTurn(conversationId, message, own.signal)
				for await (const event of events) {
					if (event.type === 'error') {
						failure = streamErrorMessage(event.code, event.message)
						continue
					}
					setState((current) => ({
						...current,
						draft: applyTurnEvent(current.draft, event),
					}))
				}
			} catch (error) {
				if (!own.signal.aborted) failure = chatErrorMessage(error)
			}
			if (controller.current !== own) return
			controller.current = null
			await refresh()
			setState(failure === undefined ? IDLE : { ...IDLE, error: failure })
		},
		[conversationId, refresh],
	)

	const abort = useCallback(() => controller.current?.abort(), [])

	const dismissError = useCallback(
		() => setState((current) => ({ ...current, error: undefined })),
		[],
	)

	return { state, send, abort, dismissError }
}
