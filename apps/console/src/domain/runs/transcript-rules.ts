export const TITLE_MAX_LENGTH = 200
export const BODY_MAX_LENGTH = 200000

export function validateTitle(value: string): string | undefined {
	const length = value.trim().length
	if (length === 0) return 'Le titre est obligatoire.'
	if (length > TITLE_MAX_LENGTH) {
		return `Le titre ne doit pas dépasser ${TITLE_MAX_LENGTH} caractères.`
	}
	return undefined
}

export function validateBody(value: string): string | undefined {
	if (value.trim().length === 0) return 'Le transcript est obligatoire.'
	if (value.length > BODY_MAX_LENGTH) {
		return `Le transcript ne doit pas dépasser ${BODY_MAX_LENGTH} caractères.`
	}
	return undefined
}
