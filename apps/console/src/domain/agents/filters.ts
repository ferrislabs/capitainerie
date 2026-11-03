import type { Schemas } from '#/api/api.client'
import {
	type AgentSource,
	agentInfo,
	agentModelChoice,
} from '#/domain/agents/catalog'
import type { ModelAssignments } from '#/domain/settings/roles'

export interface AgentRow {
	agent: Schemas.AgentResponse
	name: string
	description: string | undefined
	source: AgentSource | undefined
	model: string | undefined
	modelFallback: boolean
}

export interface AgentFilters {
	sources: ReadonlyArray<string>
	models: ReadonlyArray<string>
	query: string
}

export const NO_FILTERS: AgentFilters = { sources: [], models: [], query: '' }

export function buildRows(
	agents: ReadonlyArray<Schemas.AgentResponse>,
	assignments: ModelAssignments | null | undefined,
): ReadonlyArray<AgentRow> {
	return agents.map((agent) => {
		const info = agentInfo(agent.agent)
		const choice = agentModelChoice(agent.agent, assignments)
		return {
			agent,
			name: info?.name ?? agent.agent,
			description: info?.description,
			source: info?.source,
			model: choice?.model,
			modelFallback: choice?.fallback ?? false,
		}
	})
}

export function modelLabel(row: AgentRow): string {
	if (row.model === undefined) return '—'
	return row.modelFallback ? `${row.model} (Rédacteur)` : row.model
}

function fold(text: string): string {
	return text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase()
}

function matches(row: AgentRow, needle: string): boolean {
	return [row.name, row.description, row.agent.agent, row.model].some(
		(field) => field !== undefined && fold(field).includes(needle),
	)
}

export function filterRows(
	rows: ReadonlyArray<AgentRow>,
	filters: AgentFilters,
): ReadonlyArray<AgentRow> {
	const needle = fold(filters.query.trim())
	return rows.filter(
		(row) =>
			(filters.sources.length === 0 ||
				(row.source !== undefined && filters.sources.includes(row.source))) &&
			(filters.models.length === 0 ||
				(row.model !== undefined && filters.models.includes(row.model))) &&
			(needle === '' || matches(row, needle)),
	)
}

export function optionsOf(
	rows: ReadonlyArray<AgentRow>,
	field: 'source' | 'model',
): ReadonlyArray<string> {
	const values = new Set<string>()
	for (const row of rows) {
		const value = row[field]
		if (value !== undefined) values.add(value)
	}
	return [...values].sort((a, b) => a.localeCompare(b, 'fr'))
}

export function pruneFilters(
	filters: AgentFilters,
	rows: ReadonlyArray<AgentRow>,
): AgentFilters {
	const sources = optionsOf(rows, 'source')
	const models = optionsOf(rows, 'model')
	return {
		...filters,
		sources: filters.sources.filter((value) => sources.includes(value)),
		models: filters.models.filter((value) => models.includes(value)),
	}
}
