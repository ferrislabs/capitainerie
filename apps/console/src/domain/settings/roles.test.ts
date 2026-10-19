import { describe, expect, it } from 'vitest'
import {
	compatibleModels,
	completeAssignments,
	draftFrom,
	executorModels,
	groupByProvider,
	modelRefKey,
	parseModelRefKey,
} from '#/domain/settings/roles'
import {
	ANTHROPIC_OTHER,
	BGE,
	CLAUDE_HAIKU,
	CLAUDE_LOOKALIKE,
	MISTRAL_EMBED,
	QWEN,
	SONNET,
	VOXTRAL,
	WHISPER,
} from '#/domain/settings/test-fixtures'

const CATALOG = [QWEN, BGE, SONNET, MISTRAL_EMBED]

describe('compatibleModels', () => {
	it('offers chat models to chat roles', () => {
		for (const role of ['extractor', 'judge', 'writer'] as const) {
			expect(compatibleModels(CATALOG, role)).toEqual([QWEN, SONNET])
		}
	})

	it('offers embeddings models to the embeddings role', () => {
		expect(compatibleModels(CATALOG, 'embeddings')).toEqual([
			BGE,
			MISTRAL_EMBED,
		])
	})
})

describe('groupByProvider', () => {
	it('groups in provider order and drops empty groups', () => {
		expect(groupByProvider(CATALOG)).toEqual([
			{ provider: 'anthropic', items: [SONNET] },
			{ provider: 'mistral', items: [MISTRAL_EMBED] },
			{ provider: 'openai_compatible', items: [QWEN, BGE] },
		])
	})
})

describe('model ref keys', () => {
	it('round-trips model ids that contain a colon', () => {
		const ref = {
			provider: 'openai_compatible',
			model: 'qwen2.5:7b-instruct',
		} as const
		expect(parseModelRefKey(modelRefKey(ref))).toEqual(ref)
	})

	it('rejects unknown providers and empty keys', () => {
		expect(parseModelRefKey('')).toBeUndefined()
		expect(parseModelRefKey('openai:gpt')).toBeUndefined()
		expect(parseModelRefKey('mistral:')).toBeUndefined()
	})
})

describe('completeAssignments', () => {
	it('needs all four roles', () => {
		const ref = { provider: 'mistral', model: 'm' } as const
		expect(completeAssignments({ extractor: ref })).toBeUndefined()
		expect(
			completeAssignments({
				extractor: ref,
				judge: ref,
				writer: ref,
				embeddings: ref,
			}),
		).toEqual({
			extractor: ref,
			judge: ref,
			writer: ref,
			embeddings: ref,
			transcriber: null,
		})
	})
})

describe('executorModels', () => {
	it('keeps only Anthropic chat models whose id starts with claude-', () => {
		expect(
			executorModels([
				QWEN,
				BGE,
				SONNET,
				CLAUDE_HAIKU,
				ANTHROPIC_OTHER,
				CLAUDE_LOOKALIKE,
				MISTRAL_EMBED,
			]),
		).toEqual([SONNET, CLAUDE_HAIKU])
	})
})

describe('completeAssignments with an executor', () => {
	const ref = { provider: 'anthropic', model: 'claude-sonnet-5' } as const
	const roles = { extractor: ref, judge: ref, writer: ref, embeddings: ref }

	it('carries the executor only when one is chosen', () => {
		expect(
			completeAssignments({ ...roles, executor: 'claude-haiku-5' }),
		).toEqual({ ...roles, transcriber: null, executor: 'claude-haiku-5' })
		expect(
			completeAssignments({ ...roles, executor: null }),
		).not.toHaveProperty('executor')
	})
})

describe('the transcriber role', () => {
	it('offers transcription models of mistral and openai_compatible only', () => {
		const anthropicOne = {
			...SONNET,
			capabilities: { chat: true, embeddings: false, transcription: true },
		}
		expect(
			compatibleModels(
				[QWEN, SONNET, anthropicOne, VOXTRAL, WHISPER],
				'transcriber',
			),
		).toEqual([VOXTRAL, WHISPER])
	})

	it('is carried only when chosen, and may be null in a saved set', () => {
		const ref = { provider: 'mistral', model: 'm' } as const
		const roles = { extractor: ref, judge: ref, writer: ref, embeddings: ref }
		const voxtral = {
			provider: 'mistral',
			model: 'voxtral-mini-latest',
		} as const

		expect(completeAssignments({ ...roles, transcriber: voxtral })).toEqual({
			...roles,
			transcriber: voxtral,
		})
		expect(completeAssignments(roles)).toEqual({ ...roles, transcriber: null })
		expect(draftFrom({ ...roles, transcriber: null })).toEqual(roles)
		expect(draftFrom({ ...roles, transcriber: voxtral })).toMatchObject({
			transcriber: voxtral,
		})
		expect(draftFrom(null)).toEqual({})
	})
})
