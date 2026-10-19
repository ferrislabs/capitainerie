import type { Schemas } from '#/api/api.client'

export type LinkPatch = Pick<
	Schemas.PatchCardRequest,
	'client_id' | 'project_id'
>

type Choice = { field: 'client' | 'project'; id: string }

export function linkPatch(
	card: Schemas.CardResponse,
	choice: Choice,
	projects: ReadonlyArray<Schemas.ProjectResponse>,
): LinkPatch {
	const id = choice.id === '' ? null : choice.id
	if (choice.field === 'project') return { project_id: id }
	const current = card.project
	if (!current) return { client_id: id }
	const owner = projects.find((project) => project.id === current.id)?.client
		?.id
	return id !== null && owner === id
		? { client_id: id }
		: { client_id: id, project_id: null }
}

export function projectOptions(
	card: Schemas.CardResponse,
	projects: ReadonlyArray<Schemas.ProjectResponse>,
): ReadonlyArray<{ id: string; name: string }> {
	const clientId = card.client?.id
	const listed = projects
		.filter((project) => !clientId || project.client?.id === clientId)
		.map((project) => ({ id: project.id, name: project.name }))
	const current = card.project
	return current && !listed.some((project) => project.id === current.id)
		? [{ id: current.id, name: current.name }, ...listed]
		: listed
}

export function clientOptions(
	card: Schemas.CardResponse,
	clients: ReadonlyArray<Schemas.ClientResponse>,
): ReadonlyArray<{ id: string; name: string }> {
	const listed = clients.map((client) => ({ id: client.id, name: client.name }))
	const current = card.client
	return current && !listed.some((client) => client.id === current.id)
		? [{ id: current.id, name: current.name }, ...listed]
		: listed
}
