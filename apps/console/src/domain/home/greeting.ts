import type { Schemas } from '#/api/api.client'

const EVENING_HOUR = 18

export function greeting(now: Date): 'Bonjour' | 'Bonsoir' {
	return now.getHours() >= EVENING_HOUR ? 'Bonsoir' : 'Bonjour'
}

export function firstName(
	me: Pick<Schemas.MeResponse, 'display_name' | 'username'> | undefined,
): string | undefined {
	const display = me?.display_name?.trim().split(/\s+/)[0]
	if (display) return display
	const username = me?.username?.trim()
	return username || undefined
}

export function greetingTitle(now: Date, name: string | undefined): string {
	return name ? `${greeting(now)}, ${name}` : greeting(now)
}
