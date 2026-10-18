const DETAIL_PATTERN =
	/^(\d+) action\(s\) anchored in the transcript, (\d+) dropped \(quote not found\)$/
const COMMENT_PATTERN = /^comment on card (\S+)$/

/**
 * Le worker rédige certains résumés en anglais. On traduit à l'affichage les
 * formes connues ; tout autre texte passe tel quel.
 */
export function stepDetailText(detail: string): string {
	const match = DETAIL_PATTERN.exec(detail)
	if (!match) return detail
	const [, kept, dropped] = match
	return `${kept} action(s) ancrée(s) dans le transcript, ${dropped} écartée(s) (citation introuvable)`
}

export function toolSummaryText(summary: string): string {
	const match = COMMENT_PATTERN.exec(summary)
	return match ? `commentaire sur la carte ${match[1]}` : summary
}
