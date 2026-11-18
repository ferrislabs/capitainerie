import type { Schemas } from '#/api/api.client'

export type OwnerKind = 'organization' | 'user'

/** GitHub attend un login d'organisation : lettres, chiffres et tirets. */
export function isValidGithubLogin(login: string): boolean {
	return /^[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})$/.test(login.trim())
}

export function manifestOwner(
	kind: OwnerKind,
	login: string,
): Schemas.ManifestOwner {
	return kind === 'user'
		? { type: 'user' }
		: { type: 'organization', login: login.trim() }
}

/**
 * Le flux « manifest » de GitHub : le navigateur poste le manifeste dans un
 * formulaire vers `action_url`, GitHub crée l'application puis renvoie vers la
 * console avec un `code`.
 */
export function postManifest(
	{
		action_url,
		manifest,
	}: Pick<Schemas.ManifestResponse, 'action_url' | 'manifest'>,
	doc: Document = document,
): void {
	const form = doc.createElement('form')
	form.method = 'post'
	form.action = action_url
	form.style.display = 'none'
	const field = doc.createElement('input')
	field.type = 'hidden'
	field.name = 'manifest'
	field.value = manifest
	form.appendChild(field)
	doc.body.appendChild(form)
	form.submit()
}
