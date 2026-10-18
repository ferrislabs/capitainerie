import { describe, expect, it } from 'vitest'
import {
	compatibleModels,
	completeAssignments,
	groupByProvider,
	modelRefKey,
	parseModelRefKey,
} from '#/domain/settings/roles'
import {
	BGE,
	MISTRAL_EMBED,
	QWEN,
	SONNET,
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
		).toEqual({ extractor: ref, judge: ref, writer: ref, embeddings: ref })
	})
})
