import { PERIODS, type Period } from '#/domain/metrics/model'

/** Paramètres d'URL de la page : ce qui n'a pas sa valeur par défaut. */
export interface MetricsSearch {
	periode?: Period
	/** `AAAA-MM-JJ`, début inclus d'une période libre. */
	du?: string
	/** `AAAA-MM-JJ`, fin incluse d'une période libre. */
	au?: string
	agent?: string
	tache?: string
}

export const DEFAULT_PERIOD: Period = '30j'

const DAY = /^\d{4}-\d{2}-\d{2}$/

const isDay = (value: unknown): value is string =>
	typeof value === 'string' &&
	DAY.test(value) &&
	!Number.isNaN(Date.parse(`${value}T00:00:00Z`))

const text = (value: unknown): string | undefined =>
	typeof value === 'string' && value !== '' ? value : undefined

export function parseMetricsSearch(
	search: Record<string, unknown>,
): MetricsSearch {
	const period = PERIODS.find((item) => item.value === search.periode)?.value
	const du = isDay(search.du) ? search.du : undefined
	const au = isDay(search.au) ? search.au : undefined
	const free =
		period === 'libre' && du !== undefined && au !== undefined && du <= au
	const parsed: MetricsSearch = {
		periode: free ? 'libre' : period === 'libre' ? undefined : period,
		du: free ? du : undefined,
		au: free ? au : undefined,
		agent: text(search.agent),
		tache: text(search.tache),
	}
	if (parsed.periode === DEFAULT_PERIOD) parsed.periode = undefined
	return Object.fromEntries(
		Object.entries(parsed).filter(([, value]) => value !== undefined),
	) as MetricsSearch
}
