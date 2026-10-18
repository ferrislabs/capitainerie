export const ADMIN_ROLE = 'capitainerie-admin'

function decodeBase64Url(segment: string): string {
	const base64 = segment.replace(/-/g, '+').replace(/_/g, '/')
	const padded = base64.padEnd(
		base64.length + ((4 - (base64.length % 4)) % 4),
		'=',
	)
	const binary = atob(padded)
	const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0))
	return new TextDecoder().decode(bytes)
}

/** Rôles du realm (`realm_access.roles`) portés par un jeton FerrisKey ; vide si illisible. */
export function realmRoles(accessToken: string | null): ReadonlyArray<string> {
	if (!accessToken) return []
	const payload = accessToken.split('.')[1]
	if (!payload) return []
	try {
		const claims: unknown = JSON.parse(decodeBase64Url(payload))
		if (typeof claims !== 'object' || claims === null) return []
		const realmAccess = (claims as { realm_access?: { roles?: unknown } })
			.realm_access
		const roles = realmAccess?.roles
		return Array.isArray(roles)
			? roles.filter((role): role is string => typeof role === 'string')
			: []
	} catch {
		return []
	}
}

export function isAdminToken(accessToken: string | null): boolean {
	return realmRoles(accessToken).includes(ADMIN_ROLE)
}
