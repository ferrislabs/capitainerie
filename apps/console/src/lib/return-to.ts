export const DEFAULT_RETURN_TO = '/'

interface ReturnToState {
	returnTo?: unknown
}

export function returnToState(location: Location): ReturnToState {
	return { returnTo: `${location.pathname}${location.search}` }
}

export function safeReturnTo(state: unknown): string {
	const returnTo = (state as ReturnToState | null | undefined)?.returnTo
	if (typeof returnTo !== 'string') return DEFAULT_RETURN_TO
	if (!returnTo.startsWith('/') || returnTo.startsWith('//'))
		return DEFAULT_RETURN_TO
	if (returnTo.startsWith('/callback')) return DEFAULT_RETURN_TO
	return returnTo
}
