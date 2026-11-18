import type { TeamAgent } from '#/domain/agent-team/team-agent'

export interface OrgNode {
	id: string
	depth: number
	subCount: number
}

export function subAgents(
	agents: ReadonlyArray<TeamAgent>,
	agentId: string,
): TeamAgent[] {
	return agents.filter((a) => a.id !== agentId && a.reports_to === agentId)
}

export function ancestorsOf(
	agents: ReadonlyArray<TeamAgent>,
	agentId: string,
): TeamAgent[] {
	const chain: TeamAgent[] = []
	let current = agents.find((a) => a.id === agentId)?.reports_to ?? null
	while (current && chain.length < 16) {
		const parent = agents.find((a) => a.id === current)
		if (!parent || parent.id === agentId || chain.includes(parent)) break
		chain.push(parent)
		current = parent.reports_to ?? null
	}
	return chain
}

export function buildOrgTree(agents: ReadonlyArray<TeamAgent>): OrgNode[] {
	const ids = new Set(agents.map((a) => a.id))
	const roots = agents.filter((a) => !a.reports_to || !ids.has(a.reports_to))
	const nodes: OrgNode[] = []
	const seen = new Set<string>()
	const walk = (id: string, depth: number) => {
		if (seen.has(id)) return
		seen.add(id)
		const subs = subAgents(agents, id).filter((sub) => !seen.has(sub.id))
		nodes.push({ id, depth, subCount: subs.length })
		for (const sub of subs) walk(sub.id, depth + 1)
	}
	for (const root of roots) walk(root.id, 0)
	for (const agent of agents) walk(agent.id, 0)
	return nodes
}
