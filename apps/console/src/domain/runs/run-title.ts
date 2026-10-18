import type { Schemas } from '#/api/api.client'

export function runTitle(run: Schemas.RunResponse): string {
	const title = run.title?.trim() || run.transcript_title?.trim()
	if (title) return title
	if (run.repository) return `Exécution sur ${run.repository}`
	return `Run ${run.id.slice(0, 8)}`
}

export function isCardRun(run: Schemas.RunResponse): boolean {
	return run.card_id != null
}
