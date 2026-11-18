import { useCallback, useState } from 'react'

export const CURRENT_PROJECT_STORAGE_KEY = 'capitainerie.current-project'

function read(): string {
	try {
		return window.localStorage.getItem(CURRENT_PROJECT_STORAGE_KEY) ?? ''
	} catch {
		return ''
	}
}

/**
 * Le projet sur lequel l’utilisateur travaille, retenu d’une visite à l’autre.
 * Chaîne vide : aucun projet choisi.
 */
export function useCurrentProject(): [string, (projectId: string) => void] {
	const [projectId, setProjectId] = useState(read)
	const update = useCallback((next: string) => {
		setProjectId(next)
		try {
			if (next === '') {
				window.localStorage.removeItem(CURRENT_PROJECT_STORAGE_KEY)
			} else {
				window.localStorage.setItem(CURRENT_PROJECT_STORAGE_KEY, next)
			}
		} catch {
			// Stockage indisponible : le choix reste en mémoire.
		}
	}, [])
	return [projectId, update]
}

/** Le projet retenu s’il existe encore et n’est pas archivé, sinon aucun. */
export function resolveCurrentProject(
	projectId: string,
	projects: ReadonlyArray<{ id: string; archived: boolean }>,
): string {
	return projects.some(
		(project) => project.id === projectId && !project.archived,
	)
		? projectId
		: ''
}
