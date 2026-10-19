export const NAME_MAX_LENGTH = 120
export const ALIAS_MAX_COUNT = 10
export const TEXT_MAX_LENGTH = 5000

export function parseAliases(text: string): string[] {
	const seen = new Set<string>()
	const aliases: string[] = []
	for (const part of text.split(/[\n,]/)) {
		const alias = part.trim()
		if (alias === '' || seen.has(alias.toLowerCase())) continue
		seen.add(alias.toLowerCase())
		aliases.push(alias)
	}
	return aliases
}

export function formatAliases(aliases: ReadonlyArray<string>): string {
	return aliases.join(', ')
}

export function validateName(name: string): string | undefined {
	const trimmed = name.trim()
	if (trimmed === '') return 'Le nom est obligatoire.'
	if ([...trimmed].length > NAME_MAX_LENGTH) {
		return `Le nom ne doit pas dépasser ${NAME_MAX_LENGTH} caractères.`
	}
	return undefined
}

export function validateAliases(
	aliases: ReadonlyArray<string>,
): string | undefined {
	if (aliases.length > ALIAS_MAX_COUNT) {
		return `${ALIAS_MAX_COUNT} alias au plus.`
	}
	if (aliases.some((alias) => [...alias].length > NAME_MAX_LENGTH)) {
		return `Chaque alias tient en ${NAME_MAX_LENGTH} caractères au plus.`
	}
	return undefined
}

export function validateText(text: string): string | undefined {
	return [...text].length > TEXT_MAX_LENGTH
		? `Le texte ne doit pas dépasser ${TEXT_MAX_LENGTH} caractères.`
		: undefined
}
