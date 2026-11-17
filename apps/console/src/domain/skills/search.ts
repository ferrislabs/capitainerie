import type { DemoSkill } from '#/domain/skills/mock/skills'

export function filterSkills(
	skills: ReadonlyArray<DemoSkill>,
	query: string,
): DemoSkill[] {
	const needle = query.trim().toLowerCase()
	if (!needle) return [...skills]
	return skills.filter((s) => s.name.toLowerCase().includes(needle))
}
