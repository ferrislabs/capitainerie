import { createFileRoute } from '@tanstack/react-router'
import { SkillsPage } from '#/domain/skills/feature/skills-page'

export const Route = createFileRoute('/_app/skills')({
	component: SkillsPage,
})
