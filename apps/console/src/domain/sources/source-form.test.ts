import { describe, expect, it } from 'vitest'
import {
	exampleTitle,
	initialForm,
	parseList,
	SOURCE_TYPES,
	slugify,
	switchType,
	toSource,
	validateForm,
} from '#/domain/sources/source-form'

describe('source form', () => {
	it('lists six types', () => {
		expect(SOURCE_TYPES.map((t) => t.id)).toEqual([
			'DNS',
			'Webhook',
			'Mail',
			'RSS',
			'Git',
			'Odoo',
		])
	})

	it('slugifies names', () => {
		expect(slugify('Pièces reçues sur Chatwoot !')).toBe(
			'pieces-recues-sur-chatwoot',
		)
	})

	it('parses comma separated lists', () => {
		expect(parseList(' a , ,b ')).toEqual(['a', 'b'])
	})

	it('resets type defaults but keeps the name and agent', () => {
		const form = { ...initialForm('veille-legale'), name: 'Ma source' }
		const next = switchType(form, 'Mail')
		expect(next.name).toBe('Ma source')
		expect(next.agentId).toBe('veille-legale')
		expect(next.patterns).toBe('cabinet@natalia.fr')
		expect(next.exclusions).toBe('')
		expect(next.feed).toBe('IMAP (boîte partagée)')
		expect(next.template).toBe('Mail reçu : {valeur}')
		expect(next.mode).toBe('Une tâche par événement')
	})

	it('validates name and a pattern', () => {
		const form = initialForm('x')
		expect(validateForm(form)).toBe('Renseigne un nom et au moins un motif')
		expect(validateForm({ ...form, name: 'N' })).toBeNull()
		expect(validateForm({ ...form, name: 'N', patterns: ' , ' })).not.toBeNull()
	})

	it('builds the example title', () => {
		const form = initialForm('x')
		expect(exampleTitle(form)).toBe(
			"Sous-domaine iam.exemple.fr : identifier l'entreprise dépositaire et ses informations légales",
		)
	})

	it('builds a source with a unique slug id', () => {
		const form = { ...initialForm('a'), name: 'Ma source' }
		const s = toSource(form, [])
		expect(s).toMatchObject({
			id: 'ma-source',
			enabled: true,
			agentIds: ['a'],
			patterns: ['iam.*', 'auth.*'],
			eventsLast24h: 0,
			last: 'jamais',
		})
		expect(toSource(form, ['ma-source']).id).toBe('ma-source-2')
	})
})
