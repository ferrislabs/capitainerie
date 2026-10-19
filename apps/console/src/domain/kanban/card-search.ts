export interface CardSearch {
	card?: string | undefined
}

export function parseCardSearch(search: Record<string, unknown>): CardSearch {
	const card = search.card
	return {
		card: typeof card === 'string' && card.trim() !== '' ? card : undefined,
	}
}
