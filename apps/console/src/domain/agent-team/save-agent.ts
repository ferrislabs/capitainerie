import type { Schemas } from '#/api/api.client'
import {
	type AgentForm,
	type ConfigFields,
	describeChanges,
	diffForm,
} from '#/domain/agent-team/agent-config'
import {
	getAgentConfig,
	updateAgentConfig,
} from '#/domain/agent-team/agent-config-store'
import { DIRECTION } from '#/domain/agent-team/constants'
import type { TeamAgent } from '#/domain/agent-team/team-agent'

export function agentPatch(
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
	return patch
}

export function recordRevision(
	agent: TeamAgent,
	base: AgentForm,
	next: AgentForm,
): number {
	const changed = diffForm(base, next)
	const config = getAgentConfig(agent.id, agent)
	const fields = Object.fromEntries(
		Object.keys(config.fields).map((key) => [
			key,
			next[key as keyof AgentForm],
		]),
	) as ConfigFields
	const rev = config.rev + 1
	updateAgentConfig(agent.id, {
		fields,
		rev,
		revs: [
			{
				label: `Rév. ${rev}`,
				author: 'Baptiste Parmantier',
				when: "à l'instant",
				changed: describeChanges(changed),
			},
			...config.revs,
		],
	})
	return rev
}
