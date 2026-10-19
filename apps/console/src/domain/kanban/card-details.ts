import type { Schemas } from '#/api/api.client'

export type Priority = Schemas.PriorityDto
export type Estimate = Schemas.EstimateDto
export type Criterion = Schemas.CriterionDto

export const PRIORITIES: ReadonlyArray<Priority> = [
	'urgent',
	'high',
	'normal',
	'low',
]

export const PRIORITY_LABELS: Record<Priority, string> = {
	urgent: 'Urgente',
	high: 'Haute',
	normal: 'Normale',
	low: 'Basse',
}

export const ESTIMATES: ReadonlyArray<Estimate> = ['xs', 's', 'm', 'l', 'xl']

export const ESTIMATE_LABELS: Record<Estimate, string> = {
	xs: 'XS',
	s: 'S',
	m: 'M',
	l: 'L',
	xl: 'XL',
}

export const VISIBLE_LABELS = 2

export function isFlagged(priority: Priority): boolean {
	return priority === 'urgent' || priority === 'high'
}

function localDay(now: Date): string {
	const month = String(now.getMonth() + 1).padStart(2, '0')
	const day = String(now.getDate()).padStart(2, '0')
	return `${now.getFullYear()}-${month}-${day}`
}

export function isOverdue(
	dueDate: string | null | undefined,
	now: Date = new Date(),
): boolean {
	return Boolean(dueDate) && (dueDate as string) < localDay(now)
}

export function formatDueDate(dueDate: string): string {
	const [year, month, day] = dueDate.split('-').map(Number)
	if (!year || !month || !day) return dueDate
	return new Intl.DateTimeFormat('fr-FR', {
		day: 'numeric',
		month: 'short',
		year: 'numeric',
	}).format(new Date(year, month - 1, day))
}

export function initials(name: string): string {
	const words = name
		.trim()
		.split(/[\s-]+/)
		.filter((word) => word !== '')
	if (words.length === 0) return '?'
	const letters =
		words.length === 1
			? Array.from(words[0] ?? '').slice(0, 2)
			: [words[0], words[words.length - 1]].map(
					(word) => Array.from(word ?? '')[0],
				)
	return letters.join('').toLocaleUpperCase('fr-FR')
}

export function criteriaProgress(
	criteria: ReadonlyArray<Criterion>,
): { done: number; total: number } | null {
	if (criteria.length === 0) return null
	return {
		done: criteria.filter((criterion) => criterion.done).length,
		total: criteria.length,
	}
}

export function splitLabels(
	labels: ReadonlyArray<string>,
	visible: number = VISIBLE_LABELS,
): { shown: ReadonlyArray<string>; hidden: number } {
	return {
		shown: labels.slice(0, visible),
		hidden: Math.max(0, labels.length - visible),
	}
}

export function assigneeName(
	subject: string | null | undefined,
	names: ReadonlyMap<string, string>,
): string | undefined {
	return subject ? (names.get(subject) ?? subject) : undefined
}

const SHORT_SUBJECT_CHARS = 8

export function memberLabels(
	members: ReadonlyArray<{
		subject: string
		name: string
		email?: string | null
	}>,
): ReadonlyMap<string, string> {
	const counts = new Map<string, number>()
	for (const member of members) {
		const key = member.name.toLocaleLowerCase('fr-FR')
		counts.set(key, (counts.get(key) ?? 0) + 1)
	}
	return new Map(
		members.map((member) => {
			const shared =
				(counts.get(member.name.toLocaleLowerCase('fr-FR')) ?? 0) > 1
			const hint = member.email || member.subject.slice(0, SHORT_SUBJECT_CHARS)
			return [member.subject, shared ? `${member.name} (${hint})` : member.name]
		}),
	)
}
