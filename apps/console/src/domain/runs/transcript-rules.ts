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

export const PARTICIPANTS_MAX = 50
export const PARTICIPANT_MAX_LENGTH = 100

export function mergeParticipants(
	current: readonly string[],
	names: readonly string[],
): string[] {
	const next = [...current]
	for (const part of names) {
		const name = part.trim()
		if (name !== '' && !next.includes(name)) next.push(name)
	}
	return next
}

export function addParticipants(
	current: readonly string[],
	raw: string,
): string[] {
	return mergeParticipants(current, raw.split(','))
}

export function validateParticipants(
	participants: readonly string[],
): string | undefined {
	if (participants.length > PARTICIPANTS_MAX) {
		return `Pas plus de ${PARTICIPANTS_MAX} participants.`
	}
	if (participants.some((name) => name.length > PARTICIPANT_MAX_LENGTH)) {
		return `Un nom ne doit pas dépasser ${PARTICIPANT_MAX_LENGTH} caractères.`
	}
	return undefined
}

export function validateRepository(value: string): string | undefined {
	const repository = value.trim()
	if (repository === '') return undefined
	if (!/^[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/.test(repository)) {
		return 'Le dépôt doit être au format propriétaire/nom.'
	}
	return undefined
}
