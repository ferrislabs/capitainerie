import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	applyTurnEvent,
	EMPTY_DRAFT,
	messageViews,
	needsAnswer,
} from '#/domain/chat/chat-state'
import { toolHeadline } from '#/domain/chat/tool-label'

describe('applyTurnEvent', () => {
	it('builds the draft from the events of a turn', () => {
		let draft = EMPTY_DRAFT
		for (const event of [
			{ type: 'delta', text: 'Je ' } as const,
			{ type: 'delta', text: 'cherche.' } as const,
			{
				type: 'tool-start',
				id: 't1',
				name: 'search_cards',
				arguments: '{}',
			} as const,
			{
				type: 'tool-end',
				id: 't1',
				name: 'search_cards',
				succeeded: true,
				latencyMs: 9,
				summary: '3 cartes trouvées',
			} as const,
			{ type: 'card', cardId: 'c1', title: 'README' } as const,
			{ type: 'card', cardId: 'c1', title: 'README' } as const,
		]) {
			draft = applyTurnEvent(draft, event)
		}

		expect(draft.text).toBe('Je cherche.')
		expect(draft.tools).toEqual([
			{
				id: 't1',
				name: 'search_cards',
				arguments: '{}',
				status: 'done',
				latencyMs: 9,
				summary: '3 cartes trouvées',
			},
		])
		expect(draft.cards).toEqual([{ cardId: 'c1', title: 'README' }])
	})

	it('marks a failed tool and ignores done and error events', () => {
		let draft = applyTurnEvent(EMPTY_DRAFT, {
			type: 'tool-start',
			id: 't1',
			name: 'propose_card',
			arguments: '{}',
		})
		draft = applyTurnEvent(draft, {
			type: 'tool-end',
			id: 't1',
			name: 'propose_card',
			succeeded: false,
			latencyMs: 1,
			summary: 'forbidden',
		})
		const after = applyTurnEvent(draft, {
			type: 'done',
			status: 'complete',
			usage: { inputTokens: 0, outputTokens: 0, costMicros: 0 },
		})

		expect(draft.tools[0].status).toBe('failed')
		expect(after).toBe(draft)
	})
})

describe('messageViews', () => {
	const base: Schemas.MessageResponse = {
		id: 'm1',
		seq: 1,
		role: 'user',
		text: 'Bonjour',
		tool_calls: [],
		cards: [],
		status: 'complete',
		created_at: '2026-10-19T10:00:00Z',
	}

	it('keeps the user and assistant messages with their tools and cards', () => {
		const views = messageViews([
			base,
			{
				...base,
				id: 'm2',
				seq: 2,
				role: 'assistant',
				text: 'Voici',
				status: 'interrupted',
				tool_calls: [
					{
						name: 'search_cards',
						arguments: '{}',
						succeeded: false,
						latency_ms: 4,
						summary: 'boom',
					},
				],
				cards: [{ card_id: 'c1', title: 'README' }],
			},
			{ ...base, id: 'm3', seq: 3, role: 'tool' },
		])

		expect(views.map((view) => view.role)).toEqual(['user', 'assistant'])
		expect(views[1].status).toBe('interrupted')
		expect(views[1].tools[0]).toMatchObject({
			status: 'failed',
			summary: 'boom',
		})
		expect(views[1].cards).toEqual([{ cardId: 'c1', title: 'README' }])
	})

	it('needs an answer only when the last message is the member’s', () => {
		expect(needsAnswer([base])).toBe(true)
		expect(needsAnswer([base, { ...base, id: 'm2', role: 'assistant' }])).toBe(
			false,
		)
		expect(needsAnswer([])).toBe(false)
	})
})

describe('toolHeadline', () => {
	const tool = (
		name: string,
		status: 'running' | 'done' | 'failed',
		summary?: string,
	) => ({ id: 'x', name, arguments: '', status, summary })

	it('says what the Captain did, in French', () => {
		expect(
			toolHeadline(tool('search_cards', 'done', '3 cartes trouvées')),
		).toBe('A cherché 3 cartes')
		expect(toolHeadline(tool('search_cards', 'done', '1 carte trouvée'))).toBe(
			'A cherché 1 carte',
		)
		expect(toolHeadline(tool('search_cards', 'done', 'aucune carte'))).toBe(
			'A cherché : aucune carte',
		)
		expect(toolHeadline(tool('search_cards', 'running'))).toBe(
			'Cherche des cartes…',
		)
		expect(toolHeadline(tool('propose_card', 'done'))).toBe(
			'A proposé une carte',
		)
		expect(toolHeadline(tool('comment_card', 'running'))).toBe(
			'Commente une carte…',
		)
		expect(toolHeadline(tool('list_repositories', 'done'))).toBe(
			'A listé les dépôts',
		)
	})

	it('flags a failure with a word and names an unknown tool', () => {
		expect(toolHeadline(tool('propose_card', 'failed'))).toBe(
			'A proposé une carte (échec)',
		)
		expect(toolHeadline(tool('mystery', 'done'))).toBe('Outil mystery')
	})
})
