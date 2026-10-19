import type { Schemas } from '#/api/api.client'
import type { ProjectValues } from '#/domain/clients/ui/project-dialog'

export function projectBody(
	values: ProjectValues,
): Schemas.CreateProjectRequest {
	return {
		name: values.name,
		aliases: values.aliases,
		description: values.description,
		...(values.clientId ? { client_id: values.clientId } : {}),
		...(values.repository ? { repository: values.repository } : {}),
	}
}

export function projectPatch(
	values: ProjectValues,
): Schemas.UpdateProjectRequest {
	return {
		name: values.name,
		aliases: values.aliases,
		description: values.description,
		client_id: values.clientId || null,
		repository: values.repository || null,
	}
}
