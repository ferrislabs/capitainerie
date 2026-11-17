import { describe, expect, it } from 'vitest'
import { filterArtifacts } from '#/domain/artifacts/filter-artifacts'
import { ARTIFACTS } from '#/domain/artifacts/mock/artifacts'

describe('filterArtifacts', () => {
	it('returns everything for a blank query', () => {
		expect(filterArtifacts('  ', ARTIFACTS)).toHaveLength(ARTIFACTS.length)
	})

	it('matches the name without caring for case or accents', () => {
		expect(filterArtifacts('SCHEMA', ARTIFACTS).map((a) => a.id)).toEqual([
			'schema-appro',
		])
	})

	it('matches the agent and the type', () => {
		expect(filterArtifacts('archibald', ARTIFACTS).map((a) => a.id)).toEqual([
			'inventaire-drive',
		])
		expect(filterArtifacts('feuille', ARTIFACTS)).toHaveLength(1)
	})

	it('returns an empty list when nothing matches', () => {
		expect(filterArtifacts('zzz', ARTIFACTS)).toEqual([])
	})
})
