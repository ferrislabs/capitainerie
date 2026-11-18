import { describe, expect, it } from 'vitest'
import {
	agentsUsing,
	createBody,
	describeSkillError,
	EMPTY_DRAFT,
	filterSkills,
	type Skill,
	sourceLabel,
	updateBody,
	validateDraft,
} from '#/domain/skills/model'
import { skill } from '#/domain/skills/test-fixtures'

const SKILLS = [
	skill('humanizer', { name: 'Humanizer', description: 'Écriture naturelle' }),
	skill('gitlab-mr', { name: 'GitLab MR', description: 'Merge requests' }),
]

describe('filterSkills', () => {
	it('returns all for an empty query', () => {
		expect(filterSkills(SKILLS, '  ')).toHaveLength(2)
	})
	it('matches slug, name and description case-insensitively', () => {
		expect(filterSkills(SKILLS, 'GITLAB').map((s) => s.slug)).toEqual([
			'gitlab-mr',
		])
		expect(filterSkills(SKILLS, 'naturelle').map((s) => s.slug)).toEqual([
			'humanizer',
		])
		expect(filterSkills(SKILLS, 'zzz')).toEqual([])
	})
})

describe('validateDraft', () => {
	const ok = { ...EMPTY_DRAFT, slug: 'revue-contrat', name: 'Revue' }
	it('accepts a kebab-case slug, a name and a manual or http source', () => {
		expect(validateDraft(ok)).toBeNull()
		expect(validateDraft({ ...ok, source: 'https://x.dev/s.md' })).toBeNull()
		expect(validateDraft({ ...ok, source: '' })).toBeNull()
	})
	it('rejects bad slugs, missing names and bad sources', () => {
		expect(validateDraft({ ...ok, slug: '' })).toMatch(/obligatoire/)
		expect(validateDraft({ ...ok, slug: 'Revue_Contrat' })).toMatch(/kebab/)
		expect(validateDraft({ ...ok, slug: 'a'.repeat(61) })).toMatch(/60/)
		expect(validateDraft({ ...ok, name: ' ' })).toMatch(/nom/)
		expect(validateDraft({ ...ok, source: 'ftp://x' })).toMatch(/source/)
	})
})

describe('bodies', () => {
	it('trims the create body and defaults the source', () => {
		expect(
			createBody({
				slug: ' a ',
				name: ' A ',
				description: ' d ',
				content: '# x',
				source: ' ',
			}),
		).toEqual({
			slug: 'a',
			name: 'A',
			description: 'd',
			content: '# x',
			source: 'manual',
		})
	})
	it('only patches changed fields', () => {
		const current = SKILLS[0] as Skill
		expect(
			updateBody(current, {
				slug: 'humanizer',
				name: 'Humanizer',
				description: 'Autre',
				content: '',
				source: 'manual',
			}),
		).toEqual({ description: 'Autre' })
	})
})

it('lists the agents using a skill', () => {
	const agents = [
		{ id: 'a', skills: ['humanizer'] },
		{ id: 'b', skills: [] },
	]
	expect(agentsUsing(agents, 'humanizer').map((a) => a.id)).toEqual(['a'])
})

it('describes API errors', () => {
	expect(describeSkillError({ status: 409 })).toMatch(/slug/)
	expect(describeSkillError({ status: 422 })).toMatch(/kebab/)
	expect(describeSkillError(new Error('x'))).toMatch(/Réessaie/)
})

it('labels the source', () => {
	expect(sourceLabel('manual')).toBe('Manuelle')
	expect(sourceLabel('https://skills.example.com/a.md')).toBe(
		'skills.example.com',
	)
})
