const DAY_MS = 86_400_000

export interface DayCell {
	/** `AAAA-MM-JJ`. */
	day: string
	inMonth: boolean
}

export interface DraftRange {
	from: string
	to: string
}

export const toDay = (date: Date): string => date.toISOString().slice(0, 10)

const parse = (day: string): Date => new Date(`${day}T00:00:00Z`)

/** Premier jour du mois décalé de `delta` mois, au format `AAAA-MM-01`. */
export function shiftMonth(month: string, delta: number): string {
	const date = parse(`${month.slice(0, 7)}-01`)
	date.setUTCMonth(date.getUTCMonth() + delta)
	return toDay(date)
}

/** Semaines du mois, du lundi au dimanche, complétées par les jours voisins. */
export function monthWeeks(month: string): DayCell[][] {
	const first = parse(`${month.slice(0, 7)}-01`)
	const offset = (first.getUTCDay() + 6) % 7
	const start = first.getTime() - offset * DAY_MS
	const weeks: DayCell[][] = []
	for (let week = 0; week < 6; week += 1) {
		const cells = Array.from({ length: 7 }, (_, index) => {
			const date = new Date(start + (week * 7 + index) * DAY_MS)
			return {
				day: toDay(date),
				inMonth: date.getUTCMonth() === first.getUTCMonth(),
			}
		})
		if (week > 0 && !cells.some((cell) => cell.inMonth)) break
		weeks.push(cells)
	}
	return weeks
}

/** Clic sur un jour : le premier fixe le début, le second la fin (dans l'ordre). */
export function pickDay(draft: DraftRange, day: string): DraftRange {
	if (!draft.from || draft.to) return { from: day, to: '' }
	return day < draft.from
		? { from: day, to: draft.from }
		: { from: draft.from, to: day }
}

export const isComplete = (draft: DraftRange): boolean =>
	draft.from !== '' && draft.to !== ''

/** Bornes affichées d'une plage en cours de choix, avec l'aperçu au survol. */
export function previewRange(
	draft: DraftRange,
	hover: string | null,
): DraftRange {
	if (!draft.from || draft.to || !hover) return draft
	return hover < draft.from
		? { from: hover, to: draft.from }
		: { from: draft.from, to: hover }
}

const LABEL = new Intl.DateTimeFormat('fr-FR', {
	day: 'numeric',
	month: 'short',
	year: 'numeric',
	timeZone: 'UTC',
})

export function formatRange(range: DraftRange): string {
	if (!range.from) return 'Choisir des dates'
	const from = LABEL.format(parse(range.from))
	return range.to ? `${from} – ${LABEL.format(parse(range.to))}` : from
}
