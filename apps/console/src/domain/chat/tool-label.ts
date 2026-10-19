import type { ToolView } from '#/domain/chat/chat-state'

const FOUND = /^(\d+) cartes? trouvées?$/

function searchHeadline(tool: ToolView): string {
	if (tool.status === 'running') return 'Cherche des cartes…'
	const found = FOUND.exec(tool.summary ?? '')
	if (found) {
		const count = Number(found[1])
		return `A cherché ${count} ${count > 1 ? 'cartes' : 'carte'}`
	}
	return tool.summary === 'aucune carte'
		? 'A cherché : aucune carte'
		: 'A cherché des cartes'
}

const FIXED: Record<string, { running: string; done: string }> = {
	propose_card: { running: 'Propose une carte…', done: 'A proposé une carte' },
	comment_card: {
		running: 'Commente une carte…',
		done: 'A commenté une carte',
	},
	list_repositories: {
		running: 'Liste les dépôts…',
		done: 'A listé les dépôts',
	},
}

export function toolHeadline(tool: ToolView): string {
	const base =
		tool.name === 'search_cards'
			? searchHeadline(tool)
			: (FIXED[tool.name]?.[tool.status === 'running' ? 'running' : 'done'] ??
				`Outil ${tool.name}`)
	return tool.status === 'failed' ? `${base} (échec)` : base
}
