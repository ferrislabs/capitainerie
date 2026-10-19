import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	chooseClient,
	chooseProject,
	intakeBody,
	projectsOf,
} from '#/domain/runs/intake-links'

function project(
	id: string,
	clientId: string | null,
	repository: string | null = null,
) {
	return {
		id,
		name: id,
		client: clientId ? { id: clientId, name: clientId } : null,
		repository,
	} as Schemas.ProjectResponse
}

const PROJECTS = [
	project('site', 'acme', 'acme/web'),
	project('app', 'globex'),
	project('interne', null),
]

describe('intake links', () => {
	it('filters the projects by client, all of them without one', () => {
		expect(projectsOf(PROJECTS, 'acme').map((p) => p.id)).toEqual(['site'])
		expect(projectsOf(PROJECTS, '')).toHaveLength(3)
	})

	it('fills the client and the repository when a project is chosen', () => {
		expect(
			chooseProject(PROJECTS, { clientId: '', projectId: '' }, 'site'),
		).toEqual({
			clientId: 'acme',
			projectId: 'site',
			repository: 'acme/web',
		})
		expect(
			chooseProject(PROJECTS, { clientId: '', projectId: '' }, 'app'),
		).toEqual({
			clientId: 'globex',
			projectId: 'app',
		})
	})

	it('clears the client for an internal project and keeps it when the project is cleared', () => {
		expect(
			chooseProject(PROJECTS, { clientId: 'acme', projectId: '' }, 'interne'),
		).toEqual({ clientId: '', projectId: 'interne' })
		expect(
			chooseProject(PROJECTS, { clientId: 'acme', projectId: 'site' }, ''),
		).toEqual({ clientId: 'acme', projectId: '' })
	})

	it('drops the chosen project when another client is picked', () => {
		const current = { clientId: 'acme', projectId: 'site' }
		expect(chooseClient(PROJECTS, current, 'globex')).toEqual({
			clientId: 'globex',
			projectId: '',
		})
		expect(chooseClient(PROJECTS, current, 'acme')).toEqual({
			clientId: 'acme',
			projectId: 'site',
		})
		expect(chooseClient(PROJECTS, current, '')).toEqual({
			clientId: '',
			projectId: 'site',
		})
	})

	it('sends only what was chosen', () => {
		expect(intakeBody({ clientId: '', projectId: '' })).toEqual({})
		expect(intakeBody({ clientId: 'acme', projectId: 'site' })).toEqual({
			client_id: 'acme',
			project_id: 'site',
		})
	})
})
