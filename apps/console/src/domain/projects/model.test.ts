import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	createBody,
	draftOf,
	joinRepository,
	progressByProject,
	splitRepository,
	updateBody,
	validateDraft,
} from '#/domain/projects/model'

const project = {
	id: 'p1',
	name: 'Site',
	description: 'd',
	repository: 'acme/site',
	client: { id: 'c1', name: 'Acme' },
} as Schemas.ProjectResponse

describe('repository', () => {
	it('splits and joins owner and name', () => {
		expect(splitRepository('acme/site')).toEqual({
			owner: 'acme',
			name: 'site',
		})
		expect(splitRepository(null)).toEqual({ owner: '', name: '' })
		expect(joinRepository({ owner: ' acme ', name: 'site' })).toBe('acme/site')
		expect(joinRepository({ owner: 'acme', name: '' })).toBeNull()
	})
})

describe('progressByProject', () => {
	it('ignores rejected cards and cards without project', () => {
		const card = (column: string, id?: string) =>
			({
				column,
				project: id ? { id, name: id } : null,
			}) as Schemas.CardResponse
		const progress = progressByProject([
			card('done', 'p1'),
			card('todo', 'p1'),
			card('rejected', 'p1'),
			card('todo'),
		])
		expect(progress.get('p1')).toBe(50)
		expect(progress.size).toBe(1)
	})
})

describe('bodies', () => {
	it('builds a create body with null for empty optionals', () => {
		expect(
			createBody({
				name: ' A ',
				description: '',
				owner: '',
				repo: '',
				clientId: '',
			}),
		).toEqual({ name: 'A', description: '', repository: null, client_id: null })
	})

	it('sends only changed fields on update, nulling the cleared client', () => {
		const draft = draftOf(project)
		expect(updateBody(project, draft)).toEqual({})
		expect(updateBody(project, { ...draft, clientId: '' })).toEqual({
			client_id: null,
		})
		expect(updateBody(project, { ...draft, repo: 'autre' })).toEqual({
			repository: 'acme/autre',
		})
	})
})

describe('validateDraft', () => {
	it('requires a name and a complete repository', () => {
		const ok = { name: 'A', description: '', owner: '', repo: '', clientId: '' }
		expect(validateDraft({ ...ok, name: ' ' })).not.toBeNull()
		expect(validateDraft({ ...ok, owner: 'acme' })).not.toBeNull()
		expect(validateDraft(ok)).toBeNull()
	})
})
