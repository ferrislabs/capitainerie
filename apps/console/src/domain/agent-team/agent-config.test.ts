import { describe, expect, it } from 'vitest'
import {
	buildDefaultConfig,
	buildForm,
	describeChanges,
	diffForm,
} from '#/domain/agent-team/agent-config'
import { DEV } from '#/test-utils/team-agents'

describe('agent config', () => {
	it('starts without invented revisions', () => {
		const config = buildDefaultConfig(DEV)
		expect(config.rev).toBe(0)
		expect(config.revs).toEqual([])
		expect(config.fields.title).toBe('Ingénieur · Natalia')
	})

	it('builds the form from the API agent', () => {
		const form = buildForm(DEV, buildDefaultConfig(DEV))
		expect(form).toMatchObject({
			name: 'Dev (cycle autonome)',
			description: 'Description Dev (cycle autonome)',
			role: 'Ingénieur',
			model: 'Claude Sonnet 5.5',
			instr: '# Dev · Natalia',
			reports: 'a-chief',
		})
		expect(
			buildForm({ ...DEV, reports_to: null }, buildDefaultConfig(DEV)).reports,
		).toBe('Aucun (direction)')
	})

	it('diffs form fields', () => {
		const base = buildForm(DEV, buildDefaultConfig(DEV))
		expect(diffForm(base, base)).toEqual([])
		const next = { ...base, model: 'claude-haiku-5-5', effort: 'Élevé' }
		expect(diffForm(base, next)).toEqual(['model', 'effort'])
	})

	it('names changes in French', () => {
		expect(describeChanges(['model', 'effort', 'pCreate'])).toBe(
			'modèle, effort de réflexion, droit de créer des agents',
		)
	})
})
