import type { Window } from '#/domain/metrics/model'

const DAY_MS = 86_400_000
const SLOT_COUNT = 6

export interface DayPoint {
	/** `AAAA-MM-JJ`, UTC. */
	day: string
	cost_micros: number
	input_tokens: number
	output_tokens: number
}

interface DayRow {
	day: string
	cost_micros: number
	input_tokens: number
	output_tokens: number
}

/** Un point par jour UTC de la fenêtre, à zéro quand l'API n'a rien renvoyé. */
export function fillDays(
	rows: ReadonlyArray<DayRow>,
	window: Window,
): DayPoint[] {
	const byDay = new Map(rows.map((row) => [row.day, row]))
	const first = Math.floor(Date.parse(window.from) / DAY_MS)
	const last = Math.floor((Date.parse(window.to) - 1) / DAY_MS)
	const points: DayPoint[] = []
	for (let index = first; index <= last; index += 1) {
		const day = new Date(index * DAY_MS).toISOString().slice(0, 10)
		points.push(
			byDay.get(day) ?? {
				day,
				cost_micros: 0,
				input_tokens: 0,
				output_tokens: 0,
			},
		)
	}
	return points
}

/** Plus petit 1, 2 ou 5 × 10^n au-dessus de la valeur : un haut d'axe lisible. */
export function niceMax(value: number): number {
	if (value <= 0) return 1
	const power = 10 ** Math.floor(Math.log10(value))
	const unit = value / power
	const step = unit <= 1 ? 1 : unit <= 2 ? 2 : unit <= 5 ? 5 : 10
	return step * power
}

export interface SliceInput {
	key: string
	label: string
	value: number
}

export interface Slice extends SliceInput {
	/** Part du total, entre 0 et 1. */
	share: number
	/** Vrai pour la part qui regroupe le reste. */
	other: boolean
}

export const OTHER_KEY = '__other__'

/** Les `max` plus grosses parts, le reste regroupé en « Autres ». */
export function topSlices(
	inputs: ReadonlyArray<SliceInput>,
	max = 5,
	otherLabel = 'Autres',
): Slice[] {
	const positive = inputs.filter((input) => input.value > 0)
	const total = positive.reduce((sum, input) => sum + input.value, 0)
	if (total <= 0) return []
	const sorted = [...positive].sort((a, b) => b.value - a.value)
	const head = sorted.slice(0, max)
	const rest = sorted.slice(max)
	const slices: Slice[] = head.map((input) => ({
		...input,
		share: input.value / total,
		other: false,
	}))
	if (rest.length > 0) {
		const value = rest.reduce((sum, input) => sum + input.value, 0)
		slices.push({
			key: OTHER_KEY,
			label: otherLabel,
			value,
			share: value / total,
			other: true,
		})
	}
	return slices
}

/**
 * Donne un emplacement de couleur à chaque clé : une clé déjà vue garde le sien,
 * une nouvelle prend le plus petit emplacement libre parmi les clés affichées.
 */
export function assignSlots(
	slots: Map<string, number>,
	keys: ReadonlyArray<string>,
): void {
	const shown = new Set(keys)
	for (const key of keys) {
		if (slots.has(key)) continue
		const taken = new Set(
			[...slots].filter(([other]) => shown.has(other)).map(([, slot]) => slot),
		)
		let slot = 0
		while (taken.has(slot) && slot < SLOT_COUNT - 1) slot += 1
		slots.set(key, slot)
	}
}

export const slotColor = (slot: number): string =>
	`var(--chart-${(slot % SLOT_COUNT) + 1})`

export interface Arc {
	key: string
	/** Longueur visible de l'arc, sur la circonférence. */
	length: number
	/** Décalage du début de l'arc. */
	offset: number
}

/** Arcs d'un anneau : un écart de `gap` entre deux parts, sauf pour une part unique. */
export function donutArcs(
	slices: ReadonlyArray<Pick<Slice, 'key' | 'share'>>,
	circumference: number,
	gap = 2,
): Arc[] {
	let start = 0
	const spacing = slices.length > 1 ? gap : 0
	return slices.map((slice) => {
		const span = slice.share * circumference
		const arc = {
			key: slice.key,
			length: Math.max(span - spacing, 0.5),
			offset: start,
		}
		start += span
		return arc
	})
}

/** Fenêtre de même durée, qui se termine où la fenêtre donnée commence. */
export function previousWindow(window: Window): Window {
	const from = Date.parse(window.from)
	const length = Date.parse(window.to) - from
	return {
		from: new Date(from - length).toISOString(),
		to: window.from,
	}
}

/** Points avec le coût et les jetons cumulés depuis le début de la fenêtre. */
export function cumulate(points: ReadonlyArray<DayPoint>): DayPoint[] {
	let cost = 0
	let input = 0
	let output = 0
	return points.map((point) => {
		cost += point.cost_micros
		input += point.input_tokens
		output += point.output_tokens
		return {
			day: point.day,
			cost_micros: cost,
			input_tokens: input,
			output_tokens: output,
		}
	})
}

/** Variation en pourcentage ; `null` quand la période de référence est vide. */
export function deltaPercent(current: number, previous: number): number | null {
	return previous > 0 ? ((current - previous) / previous) * 100 : null
}
