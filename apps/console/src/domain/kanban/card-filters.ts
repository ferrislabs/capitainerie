import type { Schemas } from '#/api/api.client'
import { PRIORITIES, PRIORITY_LABELS } from '#/domain/kanban/card-details'

export interface CardFilters {
	clients: ReadonlyArray<string>
	projects: ReadonlyArray<string>
	priorities: ReadonlyArray<string>
	assignees: ReadonlyArray<string>
	labels: ReadonlyArray<string>
}

export const NO_FILTERS: CardFilters = {
	clients: [],
	projects: [],
	priorities: [],
	assignees: [],
	labels: [],
}

export function hasFilters(filters: CardFilters): boolean {
	return (
		filters.clients.length > 0 ||
		filters.projects.length > 0 ||
		filters.priorities.length > 0 ||
		filters.assignees.length > 0 ||
		filters.labels.length > 0
	)
}

function lower(value: string): string {
	return value.toLocaleLowerCase('fr-FR')
}

export function filterCards(
	cards: ReadonlyArray<Schemas.CardResponse>,
	filters: CardFilters,
): ReadonlyArray<Schemas.CardResponse> {
	if (!hasFilters(filters)) return cards
	const labels = filters.labels.map(lower)
	return cards.filter(
		(card) =>
			(filters.clients.length === 0 ||
				(card.client != null && filters.clients.includes(card.client.name))) &&
			(filters.projects.length === 0 ||
				(card.project != null &&
					filters.projects.includes(card.project.name))) &&
			(filters.priorities.length === 0 ||
				filters.priorities.includes(PRIORITY_LABELS[card.priority])) &&
			(filters.assignees.length === 0 ||
				(card.assignee_subject != null &&
					filters.assignees.includes(card.assignee_subject))) &&
			(labels.length === 0 ||
				card.labels.some((label) => labels.includes(lower(label)))),
	)
}

function distinct(values: ReadonlyArray<string>): string[] {
	return [...new Set(values)].sort((a, b) => a.localeCompare(b, 'fr'))
}

function distinctNoCase(values: ReadonlyArray<string>): string[] {
	const first = new Map<string, string>()
	for (const value of values) {
		if (!first.has(lower(value))) first.set(lower(value), value)
	}
	return distinct([...first.values()])
}

export interface FilterOptions {
	clients: string[]
	projects: string[]
	priorities: string[]
	assignees: string[]
	labels: string[]
}

export function filterOptions(
	cards: ReadonlyArray<Schemas.CardResponse>,
	filters: CardFilters,
	assigneeLabels: ReadonlyMap<string, string> = new Map(),
): FilterOptions {
	const names = <T extends { name: string }>(
		items: ReadonlyArray<T | null | undefined>,
	) => items.flatMap((item) => (item ? [item.name] : []))
	const present = new Set(cards.map((card) => card.priority))
	return {
		clients: distinct([
			...names(cards.map((card) => card.client)),
			...filters.clients,
		]),
		projects: distinct([
			...names(cards.map((card) => card.project)),
			...filters.projects,
		]),
		priorities: [
			...PRIORITIES.filter((priority) => present.has(priority)).map(
				(priority) => PRIORITY_LABELS[priority],
			),
			...filters.priorities.filter(
				(chosen) =>
					!PRIORITIES.some(
						(priority) =>
							PRIORITY_LABELS[priority] === chosen && present.has(priority),
					),
			),
		],
		assignees: [
			...new Set([
				...cards.flatMap((card) =>
					card.assignee_subject ? [card.assignee_subject] : [],
				),
				...filters.assignees,
			]),
		].sort((a, b) =>
			(assigneeLabels.get(a) ?? a).localeCompare(
				assigneeLabels.get(b) ?? b,
				'fr',
			),
		),
		labels: distinctNoCase([
			...cards.flatMap((card) => card.labels),
			...filters.labels,
		]),
	}
}
