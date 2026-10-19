import type { Schemas } from '#/api/api.client'
import type { TurnEvent } from '#/domain/chat/turn-events'

export type ToolStatus = 'running' | 'done' | 'failed'

export interface ToolView {
	id: string
	name: string
	arguments: string
	status: ToolStatus
	latencyMs?: number
	summary?: string
}

export interface ProposalView {
	cardId: string
	title: string
}

export interface Draft {
	text: string
	tools: Array<ToolView>
	cards: Array<ProposalView>
}

export interface MessageView {
	id: string
	role: 'user' | 'assistant'
	text: string
	status: Schemas.MessageStatusDto
	tools: Array<ToolView>
	cards: Array<ProposalView>
}

export const EMPTY_DRAFT: Draft = { text: '', tools: [], cards: [] }

export function applyTurnEvent(draft: Draft, event: TurnEvent): Draft {
	switch (event.type) {
		case 'delta':
			return { ...draft, text: draft.text + event.text }
		case 'tool-start':
			return {
				...draft,
				tools: [
					...draft.tools,
					{
						id: event.id,
						name: event.name,
						arguments: event.arguments,
						status: 'running',
					},
				],
			}
		case 'tool-end':
			return {
				...draft,
				tools: draft.tools.map((tool) =>
					tool.id === event.id
						? {
								...tool,
								status: event.succeeded ? 'done' : 'failed',
								latencyMs: event.latencyMs,
								summary: event.summary,
							}
						: tool,
				),
			}
		case 'card':
			return draft.cards.some((card) => card.cardId === event.cardId)
				? draft
				: {
						...draft,
						cards: [
							...draft.cards,
							{ cardId: event.cardId, title: event.title },
						],
					}
		default:
			return draft
	}
}

export function messageViews(
	messages: ReadonlyArray<Schemas.MessageResponse>,
): Array<MessageView> {
	return messages
		.filter((message) => message.role !== 'tool')
		.map((message) => ({
			id: message.id,
			role: message.role === 'user' ? 'user' : 'assistant',
			text: message.text,
			status: message.status,
			tools: message.tool_calls.map((call, index) => ({
				id: `${message.id}-${index}`,
				name: call.name,
				arguments: call.arguments,
				status: call.succeeded ? 'done' : 'failed',
				latencyMs: call.latency_ms,
				summary: call.summary,
			})),
			cards: message.cards.map((card) => ({
				cardId: card.card_id,
				title: card.title,
			})),
		}))
}

export function needsAnswer(
	messages: ReadonlyArray<Schemas.MessageResponse>,
): boolean {
	return messages.at(-1)?.role === 'user'
}
