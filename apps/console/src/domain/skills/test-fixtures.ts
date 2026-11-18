import type { Skill } from '#/domain/skills/model'

export function skill(slug: string, extra: Partial<Skill> = {}): Skill {
	return {
		id: `s-${slug}`,
		slug,
		name: slug,
		description: '',
		content: '',
		source: 'manual',
		archived: false,
		used_by: 0,
		created_by: 'u',
		created_at: '2026-11-01T00:00:00Z',
		updated_at: '2026-11-01T00:00:00Z',
		...extra,
	}
}
