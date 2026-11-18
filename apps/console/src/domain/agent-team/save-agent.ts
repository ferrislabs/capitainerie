import type { Schemas } from '#/api/api.client'
import {
	type AgentForm,
	configOf,
	describeChanges,
	diffForm,
	type StoreField,
} from '#/domain/agent-team/agent-config'
import { DIRECTION } from '#/domain/agent-team/constants'
import type { TeamAgent } from '#/domain/agent-team/team-agent'

const SUMMARY_MAX = 2000

const STORE_FIELDS: ReadonlySet<keyof AgentForm> = new Set<StoreField>([
	'name',
	'description',
	'role',
	'model',
	'reports',
	'instr',
])

export function changeSummary(text: string): string {
	return text.length > SUMMARY_MAX ? text.slice(0, SUMMARY_MAX) : text
}

export function agentPatch(
	agent: Pick<TeamAgent, 'config'>,
	base: AgentForm,
	next: AgentForm,
): Schemas.UpdateTeamAgentRequest {
	const patch: Schemas.UpdateTeamAgentRequest = {}
	const name = next.name.trim() || base.name
	if (name !== base.name) patch.name = name
	if (next.description !== base.description)
		patch.description = next.description
	if (next.role !== base.role) patch.role = next.role
	if (next.model !== base.model) patch.model = next.model.trim()
	if (next.instr !== base.instr) patch.instructions = next.instr
	if (next.reports !== base.reports)
		patch.reports_to = next.reports === DIRECTION ? null : next.reports
	const changed = diffForm(base, next)
	if (changed.some((key) => !STORE_FIELDS.has(key)))
		patch.config = { ...agent.config, ...configOf(next) }
	if (Object.keys(patch).length > 0)
		patch.change_summary = changeSummary(describeChanges(changed))
	return patch
}
