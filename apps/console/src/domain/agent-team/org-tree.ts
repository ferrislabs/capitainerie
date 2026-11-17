import { type DemoState, shortName, subAgentIds } from '#/domain/demo'

export interface OrgNode {
	id: string
	depth: number
	subCount: number
}

/** Arbre en liste à plat : chaque racine (direction) puis ses sous-agents, indentés par `depth`. */
export function buildOrgTree(state: DemoState): OrgNode[] {
	const names = new Set(state.agents.map((a) => shortName(a.name)))
	const roots = state.agents.filter((a) => {
		const parent = state.reportsTo[a.id]
		return !parent || !names.has(parent)
	})
	const nodes: OrgNode[] = []
	const seen = new Set<string>()
	const walk = (id: string, depth: number) => {
		if (seen.has(id)) return
		seen.add(id)
		const subs = subAgentIds(state, id).filter((sub) => !seen.has(sub))
		nodes.push({ id, depth, subCount: subs.length })
		for (const sub of subs) walk(sub, depth + 1)
	}
	for (const root of roots) walk(root.id, 0)
	for (const agent of state.agents) walk(agent.id, 0)
	return nodes
}
