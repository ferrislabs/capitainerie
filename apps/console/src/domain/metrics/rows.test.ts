import { describe, expect, it } from 'vitest'
import { projectRows, userRows } from '#/domain/metrics/rows'

const figures = { input_tokens: 2, output_tokens: 1, cost_micros: 3 }

describe('projectRows', () => {
	it('names the row without a project and keeps its agents', () => {
		const [row] = projectRows([
			{
				project_id: null,
				name: null,
				...figures,
				agents: [{ agent_id: null, name: 'extractor', runs: 2, ...figures }],
			},
		])
		expect(row?.label).toBe('Sans projet')
		expect(row?.details[0]).toMatchObject({
			label: 'extractor',
			sub: 'Agent retiré',
			runs: 2,
		})
	})
})

describe('userRows', () => {
	it('lists the models of a user', () => {
		const [row] = userRows([
			{
				user: 'ann',
				...figures,
				models: [{ model: 'claude', ...figures }],
			},
		])
		expect(row?.details.map((d) => d.label)).toEqual(['claude'])
	})
})
