import { describe, expect, it } from 'vitest'
import {
	buildDefaultConfig,
	buildForm,
	describeChanges,
	diffForm,
} from '#/domain/agent-team/agent-config'
import { DEMO_AGENTS } from '#/domain/demo'

const legal = DEMO_AGENTS.find((a) => a.id === 'veille-legale')
const dev = DEMO_AGENTS.find((a) => a.id === 'dev')

describe('agent config', () => {
	it('gives the legal agent its own instructions', () => {
		if (!legal) throw new Error('missing fixture')
		expect(buildDefaultConfig(legal).fields.instr).toContain(
			'Nouveaux sous-domaines sensibles',
		)
	})

	it('gives other agents generated instructions', () => {
		if (!dev) throw new Error('missing fixture')
		expect(buildDefaultConfig(dev).fields.instr).toContain('Dev')
	})

	it('diffs form fields including env', () => {
		if (!dev) throw new Error('missing fixture')
		const base = buildForm(dev, 'Chief of Staff', buildDefaultConfig(dev))
		expect(diffForm(base, base)).toEqual([])
		const next = {
			...base,
			model: 'Claude Haiku 5.5',
			env: [
				...base.env,
				{ id: 'a', name: 'A', secret: 'chatwoot-api', version: '1' },
			],
		}
		expect(diffForm(base, next)).toEqual(['model', 'env'])
	})

	it('names changes in French', () => {
		expect(describeChanges(['model', 'effort', 'pCreate'])).toBe(
			'modèle, effort de réflexion, droit de créer des agents',
		)
	})
})
