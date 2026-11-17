export function hueOf(name: string): number {
	let hue = 7
	for (const char of name) hue = (hue * 31 + char.charCodeAt(0)) % 360
	return hue
}

export function initialOf(name: string): string {
	return name.trim().charAt(0).toUpperCase() || '?'
}

export function slugify(name: string): string {
	return name
		.normalize('NFD')
		.replace(/\p{Diacritic}/gu, '')
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
}

export function uniqueAgentId(
	name: string,
	existing: ReadonlyArray<string>,
): string {
	const base = slugify(name) || 'agent'
	if (!existing.includes(base)) return base
	let n = 2
	while (existing.includes(`${base}-${n}`)) n++
	return `${base}-${n}`
}
