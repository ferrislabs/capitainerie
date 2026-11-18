const NUMBER = new Intl.NumberFormat('fr-FR')
const USD = new Intl.NumberFormat('fr-FR', {
	style: 'currency',
	currency: 'USD',
	currencyDisplay: 'narrowSymbol',
})
const MICROS = 1_000_000

export const formatCount = (value: number): string => NUMBER.format(value)

export const formatCost = (micros: number): string =>
	USD.format(micros / MICROS).replace(/ /g, ' ')

export function formatTokens(value: number): string {
	if (value >= MICROS) return `${NUMBER.format(value / MICROS)} M`
	if (value >= 1000) return `${NUMBER.format(Math.round(value / 100) / 10)} k`
	return NUMBER.format(value)
}

export function relativeAge(iso: string, now: number = Date.now()): string {
	const seconds = Math.max(0, (now - Date.parse(iso)) / 1000)
	if (Number.isNaN(seconds)) return ''
	if (seconds < 60) return "à l'instant"
	const minutes = Math.floor(seconds / 60)
	if (minutes < 60) return `il y a ${minutes} min`
	const hours = Math.floor(minutes / 60)
	if (hours < 24) return `il y a ${hours} h`
	return `il y a ${Math.floor(hours / 24)} j`
}

export function hueOf(seed: string): number {
	let hash = 0
	for (const char of seed) hash = (hash * 31 + char.charCodeAt(0)) % 360
	return hash
}
