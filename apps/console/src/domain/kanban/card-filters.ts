import type { Schemas } from '#/api/api.client'

export interface CardFilters {
	clients: ReadonlyArray<string>
	projects: ReadonlyArray<string>
}

export const NO_FILTERS: CardFilters = { clients: [], projects: [] }

export function hasFilters(filters: CardFilters): boolean {
	return filters.clients.length > 0 || filters.projects.length > 0
}

export function filterCards(
	cards: ReadonlyArray<Schemas.CardResponse>,
	filters: CardFilters,
): ReadonlyArray<Schemas.CardResponse> {
	if (!hasFilters(filters)) return cards
	return cards.filter(
		(card) =>
			(filters.clients.length === 0 ||
				(card.client != null && filters.clients.includes(card.client.name))) &&
			(filters.projects.length === 0 ||
				(card.project != null && filters.projects.includes(card.project.name))),
	)
}

function distinct(values: ReadonlyArray<string>): string[] {
	return [...new Set(values)].sort((a, b) => a.localeCompare(b, 'fr'))
}

export function filterOptions(
	cards: ReadonlyArray<Schemas.CardResponse>,
	filters: CardFilters,
): { clients: string[]; projects: string[] } {
	const names = <T extends { name: string }>(
		items: ReadonlyArray<T | null | undefined>,
	) => items.flatMap((item) => (item ? [item.name] : []))
	return {
		clients: distinct([
			...names(cards.map((card) => card.client)),
			...filters.clients,
		]),
		projects: distinct([
			...names(cards.map((card) => card.project)),
			...filters.projects,
		]),
	}
}
