import type { KeyboardEvent } from 'react'

const KEYS = new Set(['ArrowDown', 'ArrowUp', 'Home', 'End'])

/** Flèches haut/bas, Début et Fin déplacent le focus entre les liens d'un menu. */
export function moveFocusBetweenLinks(event: KeyboardEvent<HTMLElement>): void {
	if (!KEYS.has(event.key)) return
	const links = Array.from(
		event.currentTarget.querySelectorAll<HTMLElement>('a[href]'),
	)
	if (links.length === 0) return
	event.preventDefault()
	const current = links.indexOf(document.activeElement as HTMLElement)
	const last = links.length - 1
	const next =
		event.key === 'Home'
			? 0
			: event.key === 'End'
				? last
				: event.key === 'ArrowDown'
					? current >= last
						? 0
						: current + 1
					: current <= 0
						? last
						: current - 1
	links[next]?.focus()
}
