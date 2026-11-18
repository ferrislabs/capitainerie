export const MODEL_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:/-]*$/

export const MODEL_ID_ERROR =
	'Identifiant de modèle invalide : lettres, chiffres, « . _ : / - » uniquement, sans espace (ex. claude-sonnet-5-5).'

export const MODEL_ID_PLACEHOLDER =
	'ex. claude-sonnet-5-5 (vide = modèle par défaut du fournisseur)'

export function isValidModelId(value: string): boolean {
	const trimmed = value.trim()
	return trimmed === '' || MODEL_ID_PATTERN.test(trimmed)
}
