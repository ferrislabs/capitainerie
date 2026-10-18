import { useCallback, useState } from 'react'

function read(key: string, fallback: boolean): boolean {
	try {
		const value = window.localStorage.getItem(key)
		return value === null ? fallback : value === 'true'
	} catch {
		return fallback
	}
}

export function usePersistedFlag(
	key: string,
	fallback: boolean,
): [boolean, (next: boolean) => void] {
	const [value, setValue] = useState(() => read(key, fallback))
	const update = useCallback(
		(next: boolean) => {
			setValue(next)
			try {
				window.localStorage.setItem(key, String(next))
			} catch {
				// Stockage indisponible : l'état reste en mémoire.
			}
		},
		[key],
	)
	return [value, update]
}
