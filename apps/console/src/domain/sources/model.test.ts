import { describe, expect, it } from 'vitest'
import {
	createBody,
	draftOf,
	EMPTY_DRAFT,
	filterSources,
	slugFromName,
	sourcesOfAgent,
	updateBody,
	validateDraft,
} from '#/domain/sources/model'
import { source } from '#/domain/sources/test-fixtures'

const draft = (extra = {}) => ({
	...EMPTY_DRAFT,
	name: 'Pièces Chatwoot',
	slug: 'pieces-chatwoot',
	...extra,
})

describe('sources model', () => {
	it('derives a kebab-case slug from the name', () => {
		expect(slugFromName('Pièces reçues sur Chatwoot !')).toBe(
			'pieces-recues-sur-chatwoot',
		)
		expect(slugFromName(`${'a'.repeat(59)} b`)).toBe('a'.repeat(59))
	})

	it('validates name, slug and auto-run agent', () => {
		expect(validateDraft(draft())).toBeNull()
		expect(validateDraft(draft({ name: ' ' }))).toMatch(/nom/)
		expect(validateDraft(draft({ slug: 'Pas Kebab' }))).toMatch(/kebab/)
		expect(validateDraft(draft({ autoRun: true }))).toMatch(/agent/)
		expect(
			validateDraft(draft({ autoRun: true, defaultAgentId: 'a-dev' })),
		).toBeNull()
	})

	it('builds a webhook creation body with null ids', () => {
		expect(createBody(draft({ allowWrites: true }))).toEqual({
			kind: 'webhook',
			name: 'Pièces Chatwoot',
			slug: 'pieces-chatwoot',
			description: '',
			enabled: true,
			default_agent_id: null,
			project_id: null,
			auto_run: false,
			allow_writes: false,
		})
	})

	it('patches only the changed fields and clears ids with null', () => {
		const current = source('s1', {
			name: 'Pièces Chatwoot',
			slug: 'pieces-chatwoot',
			default_agent_id: 'a-dev',
			project_id: 'p1',
		})
		expect(updateBody(current, draftOf(current))).toEqual({})
		expect(
			updateBody(current, {
				...draftOf(current),
				defaultAgentId: '',
				projectId: '',
				description: 'x',
			}),
		).toEqual({ default_agent_id: null, project_id: null, description: 'x' })
	})

	it('filters by text and by default agent', () => {
		const items = [
			source('a', { name: 'Chatwoot', default_agent_id: 'a-dev' }),
			source('b', { name: 'GitLab', description: 'merge requests' }),
		]
		expect(filterSources(items, 'merge').map((s) => s.id)).toEqual(['b'])
		expect(filterSources(items, ' ').map((s) => s.id)).toEqual(['a', 'b'])
		expect(sourcesOfAgent(items, 'a-dev').map((s) => s.id)).toEqual(['a'])
	})
})
