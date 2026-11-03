import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	type AgentRow,
	buildRows,
	filterRows,
	modelLabel,
	NO_FILTERS,
	optionsOf,
	pruneFilters,
} from '#/domain/agents/filters'
import type { ModelAssignments } from '#/domain/settings/roles'

function agent(id: string): Schemas.AgentResponse {
	return {
		agent: id,
		runs: 1,
		last_seen: '2026-10-16T20:05:19Z',
		input_tokens: 10,
		output_tokens: 5,
		cost_micros: 100,
	}
}

const ASSIGNMENTS: ModelAssignments = {
	extractor: { provider: 'anthropic', model: 'claude-sonnet' },
	judge: { provider: 'mistral', model: 'mistral-small' },
	writer: { provider: 'anthropic', model: 'claude-haiku' },
	embeddings: { provider: 'mistral', model: 'mistral-embed' },
	executor: 'claude-opus',
}

const ROWS: ReadonlyArray<AgentRow> = buildRows(
	[
		agent('capitainerie-agent-writer'),
		agent('capitainerie-agent-executor'),
		agent('capitainerie-agent-mystery'),
	],
	ASSIGNMENTS,
)

function ids(rows: ReadonlyArray<AgentRow>) {
	return rows.map((row) => row.agent.agent)
}

describe('buildRows', () => {
	it('joins catalog and assignments, with the id as fallback name', () => {
		expect(ROWS[0]?.name).toBe('Rédacteur')
		expect(ROWS[0]?.model).toBe('claude-haiku')
		expect(ROWS[2]?.name).toBe('capitainerie-agent-mystery')
		expect(ROWS[2]?.source).toBeUndefined()
		expect(ROWS[2]?.model).toBeUndefined()
	})
})

describe('buildRows with the Captain and the Intervenant unset', () => {
	const rows = buildRows(
		[
			agent('capitainerie-agent-captain'),
			agent('capitainerie-agent-integrator'),
			agent('capitainerie-agent-writer'),
		],
		{ ...ASSIGNMENTS, captain: null },
	)

	it('shows the writer model, flagged as a fallback', () => {
		expect(rows.map((row) => [row.model, row.modelFallback])).toEqual([
			['claude-haiku', true],
			['claude-haiku', true],
			['claude-haiku', false],
		])
		expect(rows.map(modelLabel)).toEqual([
			'claude-haiku (Rédacteur)',
			'claude-haiku (Rédacteur)',
			'claude-haiku',
		])
	})

	it('keeps one model entry to filter on', () => {
		expect(optionsOf(rows, 'model')).toEqual(['claude-haiku'])
		expect(
			ids(filterRows(rows, { ...NO_FILTERS, models: ['claude-haiku'] })),
		).toHaveLength(3)
	})

	it('labels a missing model with a dash', () => {
		expect(modelLabel(ROWS[2] as AgentRow)).toBe('—')
	})
})

describe('filterRows', () => {
	it('returns everything without filters', () => {
		expect(filterRows(ROWS, NO_FILTERS)).toHaveLength(3)
	})

	it('filters by source, any of the chosen values', () => {
		expect(
			ids(filterRows(ROWS, { ...NO_FILTERS, sources: ['Pipeline'] })),
		).toEqual(['capitainerie-agent-writer'])
		expect(
			filterRows(ROWS, { ...NO_FILTERS, sources: ['Pipeline', 'Exécutant'] }),
		).toHaveLength(2)
	})

	it('filters by model', () => {
		expect(
			ids(filterRows(ROWS, { ...NO_FILTERS, models: ['claude-opus'] })),
		).toEqual(['capitainerie-agent-executor'])
	})

	it('combines filters with a logical and', () => {
		expect(
			filterRows(ROWS, {
				...NO_FILTERS,
				sources: ['Pipeline'],
				models: ['claude-opus'],
			}),
		).toHaveLength(0)
	})

	it('searches name, description, id and model without case or accents', () => {
		const search = (query: string) =>
			ids(filterRows(ROWS, { ...NO_FILTERS, query }))

		expect(search('redacteur')).toEqual(['capitainerie-agent-writer'])
		expect(search('CLAUDE CODE')).toEqual(['capitainerie-agent-executor'])
		expect(search('mystery')).toEqual(['capitainerie-agent-mystery'])
		expect(search('claude-opus')).toEqual(['capitainerie-agent-executor'])
		expect(search('  ')).toHaveLength(3)
		expect(search('introuvable')).toEqual([])
	})
})

describe('pruneFilters', () => {
	it('drops selected values that no longer appear in the rows', () => {
		const pruned = pruneFilters(
			{
				sources: ['Pipeline', 'MCP'],
				models: ['gone', 'claude-opus'],
				query: 'x',
			},
			ROWS,
		)

		expect(pruned).toEqual({
			sources: ['Pipeline'],
			models: ['claude-opus'],
			query: 'x',
		})
	})
})

describe('optionsOf', () => {
	it('lists the distinct values present, sorted', () => {
		expect(optionsOf(ROWS, 'source')).toEqual(['Exécutant', 'Pipeline'])
		expect(optionsOf(ROWS, 'model')).toEqual(['claude-haiku', 'claude-opus'])
	})
})
