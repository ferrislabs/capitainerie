import { describe, expect, it } from 'vitest'
import {
	adapterNameOf,
	type Harness,
	harnessOfModel,
	hasUsableKey,
	keySourceLabel,
	keySourceOf,
} from '#/domain/agent-team/harness'

const HARNESSES: ReadonlyArray<Harness> = [
	{
		id: 'claude-code',
		name: 'Claude Code',
		provider: 'anthropic',
		default_model: 'claude-sonnet-5-5',
		models: ['claude-sonnet-5-5', 'claude-haiku-5-5'],
	},
	{
		id: 'codex',
		name: 'Codex',
		provider: 'openai',
		default_model: 'gpt-6-sol',
		models: ['gpt-6-sol', 'gpt-6-mini'],
	},
]

describe('harness', () => {
	it('finds the harness of a model', () => {
		expect(harnessOfModel(HARNESSES, 'gpt-6-mini')?.id).toBe('codex')
		expect(harnessOfModel(HARNESSES, 'claude-opus-9')?.id).toBe('claude-code')
		expect(harnessOfModel(HARNESSES, 'mystere')).toBeUndefined()
	})

	it('derives the adapter name with fallbacks', () => {
		expect(adapterNameOf(HARNESSES, 'gpt-6-sol', '')).toBe('Codex')
		expect(adapterNameOf(HARNESSES, 'x', 'Codex')).toBe('Codex')
		expect(adapterNameOf(HARNESSES, 'x', 'Cursor')).toBe('Claude Code')
	})

	it('resolves the key source', () => {
		const mine = [
			{
				kind: 'openai' as const,
				preview: 'sk-…ab',
				default_model: 'gpt-6-sol',
				is_default: true,
				updated_at: '',
			},
		]
		expect(keySourceOf('openai', mine, [])).toEqual({
			kind: 'personal',
			preview: 'sk-…ab',
		})
		expect(keySourceOf('anthropic', mine, 'forbidden').kind).toBe(
			'organization-unknown',
		)
		expect(keySourceOf('anthropic', mine, []).kind).toBe('none')
		expect(keySourceOf('anthropic', undefined, []).kind).toBe('loading')
		expect(hasUsableKey({ kind: 'none' })).toBe(false)
		expect(keySourceLabel({ kind: 'organization' }, false)).toBe(
			"Clé de l'organisation",
		)
	})
})
