import { describe, expect, it } from 'vitest'
import { DEMO_SKILLS } from '#/domain/skills/mock/skills'
import { filterSkills } from '#/domain/skills/search'

describe('filterSkills', () => {
	it('returns all for an empty query', () => {
		expect(filterSkills(DEMO_SKILLS, '  ')).toHaveLength(6)
	})
	it('matches names case-insensitively', () => {
		expect(filterSkills(DEMO_SKILLS, 'GITLAB').map((s) => s.name)).toEqual([
			'gitlab-mr',
		])
	})
	it('does not match descriptions', () => {
		expect(filterSkills(DEMO_SKILLS, 'Pennylane')).toHaveLength(1)
		expect(filterSkills(DEMO_SKILLS, 'zzz')).toEqual([])
	})
})
