import type { Schemas } from '#/api/api.client'

export interface RepositoryParts {
	owner: string
	name: string
}

export function splitRepository(
	repository: string | null | undefined,
): RepositoryParts {
	const [owner = '', ...rest] = (repository ?? '').split('/')
	return { owner, name: rest.join('/') }
}

export function joinRepository(parts: RepositoryParts): string | null {
	const owner = parts.owner.trim()
	const name = parts.name.trim()
	return owner && name ? `${owner}/${name}` : null
}

export function progressByProject(
	cards: ReadonlyArray<Schemas.CardResponse>,
): ReadonlyMap<string, number> {
	const totals = new Map<string, { done: number; total: number }>()
	for (const card of cards) {
		if (!card.project || card.column === 'rejected') continue
		const entry = totals.get(card.project.id) ?? { done: 0, total: 0 }
		entry.total += 1
		if (card.column === 'done') entry.done += 1
		totals.set(card.project.id, entry)
	}
	return new Map(
		[...totals].map(([id, { done, total }]) => [
			id,
			Math.round((done / total) * 100),
		]),
	)
}

export interface ProjectDraft {
	name: string
	description: string
	owner: string
	repo: string
	clientId: string
}

export const EMPTY_DRAFT: ProjectDraft = {
	name: '',
	description: '',
	owner: '',
	repo: '',
	clientId: '',
}

export function draftOf(project: Schemas.ProjectResponse): ProjectDraft {
	const { owner, name } = splitRepository(project.repository)
	return {
		name: project.name,
		description: project.description,
		owner,
		repo: name,
		clientId: project.client?.id ?? '',
	}
}

export function createBody(draft: ProjectDraft): Schemas.CreateProjectRequest {
	return {
		name: draft.name.trim(),
		description: draft.description.trim(),
		repository: joinRepository({ owner: draft.owner, name: draft.repo }),
		client_id: draft.clientId || null,
	}
}

export function updateBody(
	project: Schemas.ProjectResponse,
	draft: ProjectDraft,
): Schemas.UpdateProjectRequest {
	const before = draftOf(project)
	const body: Schemas.UpdateProjectRequest = {}
	if (draft.name.trim() !== before.name) body.name = draft.name.trim()
	if (draft.description.trim() !== before.description) {
		body.description = draft.description.trim()
	}
	if (
		draft.owner.trim() !== before.owner ||
		draft.repo.trim() !== before.repo
	) {
		body.repository = joinRepository({ owner: draft.owner, name: draft.repo })
	}
	if (draft.clientId !== before.clientId)
		body.client_id = draft.clientId || null
	return body
}

export function validateDraft(draft: ProjectDraft): string | null {
	if (!draft.name.trim()) return 'Donne un nom au projet'
	const hasOwner = draft.owner.trim() !== ''
	const hasRepo = draft.repo.trim() !== ''
	if (hasOwner !== hasRepo) {
		return 'Renseigne le propriétaire et le nom du dépôt, ou aucun des deux'
	}
	return null
}
