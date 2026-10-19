import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	clientOptions,
	linkPatch,
	projectOptions,
} from '#/domain/kanban/link-patch'

function project(id: string, name: string, clientId?: string) {
	return {
		id,
		name,
		client: clientId ? { id: clientId, name: clientId } : null,
	} as Schemas.ProjectResponse
}

function card(client?: string, projectId?: string) {
	return {
		id: 'c1',
		version: 3,
		client: client ? { id: client, name: client } : null,
		project: projectId ? { id: projectId, name: projectId } : null,
	} as Schemas.CardResponse
}

const PROJECTS = [project('p1', 'Site', 'acme'), project('p2', 'App', 'globex')]

describe('linkPatch', () => {
	it('sets or clears the project alone', () => {
		expect(linkPatch(card(), { field: 'project', id: 'p1' }, PROJECTS)).toEqual(
			{
				project_id: 'p1',
			},
		)
		expect(
			linkPatch(card('acme', 'p1'), { field: 'project', id: '' }, PROJECTS),
		).toEqual({ project_id: null })
	})

	it('sets the client alone when the card has no project', () => {
		expect(
			linkPatch(card(), { field: 'client', id: 'acme' }, PROJECTS),
		).toEqual({
			client_id: 'acme',
		})
	})

	it('keeps the project when its client is the one chosen', () => {
		expect(
			linkPatch(card('acme', 'p1'), { field: 'client', id: 'acme' }, PROJECTS),
		).toEqual({ client_id: 'acme' })
	})

	it('clears the project when another client, or none, is chosen', () => {
		expect(
			linkPatch(
				card('acme', 'p1'),
				{ field: 'client', id: 'globex' },
				PROJECTS,
			),
		).toEqual({ client_id: 'globex', project_id: null })
		expect(
			linkPatch(card('acme', 'p1'), { field: 'client', id: '' }, PROJECTS),
		).toEqual({ client_id: null, project_id: null })
	})
})

describe('options', () => {
	it('lists the projects of the card’s client, keeping the current one', () => {
		expect(projectOptions(card('acme'), PROJECTS).map((p) => p.id)).toEqual([
			'p1',
		])
		expect(projectOptions(card(), PROJECTS).map((p) => p.id)).toEqual([
			'p1',
			'p2',
		])
		expect(
			projectOptions(card('acme', 'p9'), PROJECTS).map((p) => p.id),
		).toEqual(['p9', 'p1'])
	})

	it('keeps an archived current client in the choices', () => {
		const clients = [{ id: 'acme', name: 'Acme' }] as Schemas.ClientResponse[]
		expect(clientOptions(card('old'), clients).map((c) => c.id)).toEqual([
			'old',
			'acme',
		])
	})
})
