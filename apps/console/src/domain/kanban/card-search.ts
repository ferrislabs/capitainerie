export interface CardSearch {
	card?: string | undefined
	client?: string[] | undefined
	project?: string[] | undefined
}

function names(value: unknown): string[] | undefined {
	const list = Array.isArray(value) ? value : [value]
	const found = list.filter(
		(item): item is string => typeof item === 'string' && item.trim() !== '',
	)
	return found.length > 0 ? [...new Set(found)] : undefined
}

export function parseCardSearch(search: Record<string, unknown>): CardSearch {
	const card = search.card
	return {
		card: typeof card === 'string' && card.trim() !== '' ? card : undefined,
		client: names(search.client),
		project: names(search.project),
	}
}
