import type { Schemas } from '#/api/api.client'

export interface Figures {
	input_tokens: number
	output_tokens: number
	cost_micros: number
}

export interface CostRow {
	key: string
	label: string
	sub?: string
	runs?: number
	figures: Figures
	details: ReadonlyArray<CostRow>
}

const NO_PROJECT = 'Sans projet'
const NO_AGENT = 'Agent retiré'

function agentRow(agent: Schemas.AgentSpendResponse): CostRow {
	return {
		key: agent.agent_id ?? `name:${agent.name}`,
		label: agent.name,
		sub: agent.agent_id ? undefined : NO_AGENT,
		runs: agent.runs,
		figures: agent,
		details: [],
	}
}

export function projectRows(
	rows: ReadonlyArray<Schemas.ProjectSpendResponse>,
): CostRow[] {
	return rows.map((row) => ({
		key: row.project_id ?? 'none',
		label: row.name ?? NO_PROJECT,
		figures: row,
		details: row.agents.map(agentRow),
	}))
}

export function agentRows(
	rows: ReadonlyArray<Schemas.AgentSpendResponse>,
): CostRow[] {
	return rows.map(agentRow)
}

export function taskRows(
	rows: ReadonlyArray<Schemas.TaskSpendResponse>,
): CostRow[] {
	return rows.map((row) => ({
		key: row.card_id,
		label: row.title,
		figures: row,
		details: row.agents.map(agentRow),
	}))
}

export function userRows(
	rows: ReadonlyArray<Schemas.UserSpendResponse>,
): CostRow[] {
	return rows.map((row) => ({
		key: row.user,
		label: row.user,
		figures: row,
		details: row.models.map((model) => ({
			key: model.model,
			label: model.model,
			figures: model,
			details: [],
		})),
	}))
}
