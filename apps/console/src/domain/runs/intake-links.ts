import type { Schemas } from '#/api/api.client'

export interface IntakeLinks {
	clientId: string
	projectId: string
}

export interface IntakeChange extends IntakeLinks {
	repository?: string
}

export function projectsOf(
	projects: ReadonlyArray<Schemas.ProjectResponse>,
	clientId: string,
): ReadonlyArray<Schemas.ProjectResponse> {
	return clientId === ''
		? projects
		: projects.filter((project) => project.client?.id === clientId)
}

export function chooseClient(
	projects: ReadonlyArray<Schemas.ProjectResponse>,
	current: IntakeLinks,
	clientId: string,
): IntakeChange {
	const kept = projects.find(
		(project) =>
			project.id === current.projectId &&
			(clientId === '' || project.client?.id === clientId),
	)
	return { clientId, projectId: kept ? kept.id : '' }
}

export function chooseProject(
	projects: ReadonlyArray<Schemas.ProjectResponse>,
	current: IntakeLinks,
	projectId: string,
): IntakeChange {
	const project = projects.find((item) => item.id === projectId)
	if (!project) return { clientId: current.clientId, projectId: '' }
	return {
		clientId: project.client?.id ?? '',
		projectId: project.id,
		...(project.repository ? { repository: project.repository } : {}),
	}
}

export function intakeBody(links: IntakeLinks): {
	client_id?: string
	project_id?: string
} {
	return {
		...(links.clientId ? { client_id: links.clientId } : {}),
		...(links.projectId ? { project_id: links.projectId } : {}),
	}
}
