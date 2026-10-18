import type { Schemas } from '#/api/api.client'

const compactFormat = new Intl.NumberFormat('fr-FR', {
	maximumFractionDigits: 1,
})
const priceFormat = new Intl.NumberFormat('fr-FR', {
	minimumFractionDigits: 2,
	maximumFractionDigits: 2,
})
const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

export const EMPTY_VALUE = '—'

/** Fenêtre de contexte : 1M, 200k, 128k (base 1024 quand la valeur en est un multiple). */
export function formatContextWindow(tokens: number | null | undefined): string {
	if (tokens == null || tokens <= 0) return EMPTY_VALUE
	const base = tokens % 1024 === 0 ? 1024 : 1000
	if (tokens >= base * base) {
		return `${compactFormat.format(tokens / (base * base))}M`
	}
	if (tokens >= base) return `${compactFormat.format(tokens / base)}k`
	return compactFormat.format(tokens)
}

/** Prix par million de tokens, stocké en micro-dollars. */
export function formatPricePerMtok(micros: number | null | undefined): string {
	if (micros == null) return EMPTY_VALUE
	return `${priceFormat.format(micros / 1_000_000)} $`
}

export function formatPrice(
	price: Schemas.PriceResponse | null | undefined,
): string {
	const input = price?.input_per_mtok_micros
	const output = price?.output_per_mtok_micros
	if (input == null && output == null) return EMPTY_VALUE
	return `${formatPricePerMtok(input)} / ${formatPricePerMtok(output)}`
}

export function formatDateTime(iso: string): string {
	const date = new Date(iso)
	return Number.isNaN(date.getTime()) ? iso : dateFormat.format(date)
}
