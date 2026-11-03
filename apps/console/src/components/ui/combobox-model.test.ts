import { describe, expect, it } from 'vitest'

import {
	buildSections,
	edgeIndex,
	flattenSections,
	matchesQuery,
	normalize,
	stepIndex,
	typeaheadIndex,
} from '#/components/ui/combobox-model'

const options = [
	{ value: 'a', label: 'Alpha' },
	{ value: 'b', label: 'Bravo', disabled: true },
	{ value: 'c', label: 'Charlie' },
	{ value: 'd', label: 'Cobalt' },
]

describe('combobox model', () => {
	it('normalizes case and accents', () => {
		expect(normalize('  Éléphant ')).toBe('elephant')
	})

	it('matches every word of the query, in labels and keywords', () => {
		const option = { value: 'x', label: 'Écureuil roux', keywords: 'rongeur' }
		expect(matchesQuery(option, 'ecu roux')).toBe(true)
		expect(matchesQuery(option, 'rongeur')).toBe(true)
		expect(matchesQuery(option, 'chat')).toBe(false)
	})

	it('keeps order and drops empty groups', () => {
		const sections = buildSections(
			[
				options[0] ?? { value: '', label: '' },
				{ label: 'G1', options: [options[2] ?? { value: '', label: '' }] },
				{ label: 'G2', options: [options[3] ?? { value: '', label: '' }] },
			],
			'char',
		)
		expect(sections.map((section) => section.label)).toEqual(['G1'])
		expect(flattenSections(sections).map((option) => option.value)).toEqual([
			'c',
		])
	})

	it('steps over disabled options and wraps', () => {
		expect(stepIndex(options, 0, 1)).toBe(2)
		expect(stepIndex(options, 3, 1)).toBe(0)
		expect(stepIndex(options, 0, -1)).toBe(3)
		expect(edgeIndex(options, 'first')).toBe(0)
		expect(edgeIndex(options, 'last')).toBe(3)
		expect(edgeIndex([], 'first')).toBe(-1)
	})

	it('finds typeahead matches and cycles on a repeated key', () => {
		expect(typeaheadIndex(options, -1, 'c')).toBe(2)
		expect(typeaheadIndex(options, 2, 'c')).toBe(3)
		expect(typeaheadIndex(options, 3, 'c')).toBe(2)
		expect(typeaheadIndex(options, 0, 'co')).toBe(3)
		expect(typeaheadIndex(options, 0, 'b')).toBe(-1)
	})
})
