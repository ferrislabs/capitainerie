import { describe, expect, it } from 'vitest'
import { projectBody, projectPatch } from '#/domain/clients/project-body'

const values = {
	name: 'Site',
	clientId: '',
	aliases: ['Refonte'],
	repository: '',
	description: '',
}

describe('project bodies', () => {
	it('leaves out an empty client and repository when creating', () => {
		expect(projectBody(values)).toEqual({
			name: 'Site',
			aliases: ['Refonte'],
			description: '',
		})
		expect(
			projectBody({ ...values, clientId: 'c1', repository: 'acme/web' }),
		).toMatchObject({ client_id: 'c1', repository: 'acme/web' })
	})

	it('sends null to clear the client and the repository when editing', () => {
		expect(projectPatch(values)).toMatchObject({
			client_id: null,
			repository: null,
		})
		expect(projectPatch({ ...values, clientId: 'c1' })).toMatchObject({
			client_id: 'c1',
		})
	})
})
