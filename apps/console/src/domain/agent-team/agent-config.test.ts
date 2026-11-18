import { describe, expect, it } from 'vitest'
import {
	buildDefaultConfig,
	buildForm,
	configOf,
	describeChanges,
	diffForm,
	parseConfig,
} from '#/domain/agent-team/agent-config'
import { DEV } from '#/test-utils/team-agents'

describe('agent config', () => {
	it('derives defaults from the agent', () => {
		const config = buildDefaultConfig(DEV)
		expect(config.title).toBe('Ingénieur')
		expect(config.adapter).toBe('Claude Code')
		expect(config.canDeleg).toBe(false)
	})

	it('parses the stored config, ignoring unknown and ill-typed keys', () => {
		const parsed = parseConfig({
			...DEV,
			config: {
				effort: 'Élevé',
				chrome: true,
				canDeleg: 'yes',
				turns: 12,
				unknown: 'x',
			},
		})
		expect(parsed.effort).toBe('Élevé')
		expect(parsed.chrome).toBe(true)
		expect(parsed.canDeleg).toBe(false)
		expect(parsed.turns).toBe('')
		expect(parsed).not.toHaveProperty('unknown')
	})

	it('builds the form from the API agent', () => {
		const form = buildForm({ ...DEV, config: { adapter: 'Codex' } })
		expect(form).toMatchObject({
			name: 'Dev (cycle autonome)',
			description: 'Description Dev (cycle autonome)',
			role: 'Ingénieur',
			model: 'Claude Sonnet 5.5',
			instr: '# Dev · Natalia',
			reports: 'a-chief',
			adapter: 'Codex',
		})
		expect(buildForm({ ...DEV, reports_to: null }).reports).toBe(
			'Aucun (direction)',
		)
	})

	it('extracts only config fields from the form', () => {
		const config = configOf(buildForm(DEV))
		expect(config).not.toHaveProperty('name')
		expect(config).not.toHaveProperty('instr')
		expect(config).toEqual(buildDefaultConfig(DEV))
	})

	it('diffs form fields', () => {
		const base = buildForm(DEV)
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
