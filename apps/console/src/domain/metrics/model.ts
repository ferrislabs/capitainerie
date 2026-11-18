import { monthWindow } from '#/domain/audit/model'

export const PERIODS = [
	{ value: '7j', label: '7 derniers jours' },
	{ value: '30j', label: '30 derniers jours' },
	{ value: 'mois', label: 'Mois en cours' },
	{ value: 'libre', label: 'Dates libres' },
] as const

export type Period = (typeof PERIODS)[number]['value']

export interface CustomRange {
	/** `AAAA-MM-JJ`, inclus. */
	from: string
	/** `AAAA-MM-JJ`, inclus. */
	to: string
}

export interface Window {
	from: string
	to: string
}

const DAY_MS = 86_400_000

function rolling(now: Date, days: number): Window {
	return {
		from: new Date(now.getTime() - days * DAY_MS).toISOString(),
		to: now.toISOString(),
	}
}

function dayStart(day: string): number {
	return Date.parse(`${day}T00:00:00Z`)
}

/** Fenêtre `[from, to[` en UTC ; `null` tant qu'une borne libre manque. */
export function periodWindow(
	period: Period,
	now: Date,
	range: CustomRange,
): Window | null {
	switch (period) {
		case '7j':
			return rolling(now, 7)
		case '30j':
			return rolling(now, 30)
		case 'mois':
			return monthWindow(now)
		case 'libre': {
			if (!range.from || !range.to) return null
			return {
				from: new Date(dayStart(range.from)).toISOString(),
				to: new Date(dayStart(range.to) + DAY_MS).toISOString(),
			}
		}
	}
}

export function isValidWindow(window: Window): boolean {
	return Date.parse(window.from) < Date.parse(window.to)
}
