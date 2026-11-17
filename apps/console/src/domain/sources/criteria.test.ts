import { describe, expect, it } from 'vitest'
import {
	CRITERIA,
	type Dossier,
	decide,
	makeCriterion,
	PRESETS,
} from '#/domain/sources/criteria'

const good: Dossier = {
	raison: 'ACME',
	siren: '512 847 391',
	indices: 3,
	active: true,
	collective: false,
	effectif: 150,
	naf: '5229A',
	pays: 'FR',
	crm: false,
	rdapMasque: false,
	age: 2400,
	task: 'NAT-1',
}

describe('catalogue', () => {
	it('has the nine criteria', () => {
		expect(Object.keys(CRITERIA).sort()).toEqual(
			[
				'active',
				'age',
				'collective',
				'crm',
				'effectif',
				'naf',
				'pays',
				'rdap',
				'siren',
			].sort(),
		)
	})

	it('evaluates siren with indices', () => {
		expect(CRITERIA.siren.evaluate(good, undefined)).toBe('ok')
		expect(CRITERIA.siren.evaluate({ ...good, indices: 1 }, undefined)).toBe(
			'unknown',
		)
		expect(CRITERIA.siren.evaluate({ ...good, siren: '' }, undefined)).toBe(
			'ko',
		)
	})

	it('evaluates effectif, naf, pays, age with a parameter', () => {
		expect(CRITERIA.effectif.evaluate(good, 50)).toBe('ok')
		expect(CRITERIA.effectif.evaluate(good, '200')).toBe('ko')
		expect(CRITERIA.effectif.evaluate({ ...good, effectif: null }, 50)).toBe(
			'unknown',
		)
		expect(CRITERIA.naf.evaluate(good, '5229A, 6201Z')).toBe('ok')
		expect(CRITERIA.naf.evaluate(good, '6201Z')).toBe('ko')
		expect(CRITERIA.pays.evaluate({ ...good, pays: 'DE' }, 'FR')).toBe('ko')
		expect(CRITERIA.age.evaluate({ ...good, age: 15 }, 30)).toBe('ko')
	})

	it('evaluates boolean criteria', () => {
		expect(
			CRITERIA.active.evaluate({ ...good, active: false }, undefined),
		).toBe('ko')
		expect(CRITERIA.collective.evaluate(good, undefined)).toBe('ok')
		expect(CRITERIA.crm.evaluate({ ...good, crm: true }, undefined)).toBe('ko')
		expect(
			CRITERIA.rdap.evaluate({ ...good, rdapMasque: true }, undefined),
		).toBe('unknown')
	})
})

describe('presets', () => {
	it('exposes the three presets', () => {
		expect(Object.keys(PRESETS)).toEqual([
			'Prospection B2B',
			'Veille sécurité',
			'Minimal',
		])
		expect(PRESETS['Prospection B2B']).toHaveLength(6)
	})
})

describe('decide', () => {
	const b2b = PRESETS['Prospection B2B']

	it('qualifies a clean dossier', () => {
		expect(decide(b2b, good).verdict).toBe('qualified')
	})

	it('rejects on a failing reject criterion', () => {
		expect(decide(b2b, { ...good, active: false }).verdict).toBe('rejected')
	})

	it('asks for review on an unknown criterion', () => {
		expect(decide(b2b, { ...good, effectif: null }).verdict).toBe('review')
	})

	it('rejection wins over review whatever the order', () => {
		const d = { ...good, indices: 1, collective: true }
		expect(decide(b2b, d).verdict).toBe('rejected')
	})

	it('never counts ignored criteria', () => {
		const pipeline = [makeCriterion('effectif', 500, 'ignore')]
		expect(decide(pipeline, good).verdict).toBe('qualified')
	})

	it('skips disabled criteria and keeps row results', () => {
		const pipeline = [
			{ ...makeCriterion('active', undefined, 'reject'), enabled: false },
			makeCriterion('siren', undefined, 'review'),
		]
		const out = decide(pipeline, { ...good, active: false })
		expect(out.verdict).toBe('qualified')
		expect(out.rows).toHaveLength(1)
		expect(out.rows[0]?.result).toBe('ok')
	})

	it('qualifies an empty pipeline', () => {
		expect(decide([], good).verdict).toBe('qualified')
	})
})
