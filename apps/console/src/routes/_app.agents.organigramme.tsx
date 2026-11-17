import { createFileRoute } from '@tanstack/react-router'
import { OrgChartPage } from '#/domain/agent-team/feature/org-chart-page'

export const Route = createFileRoute('/_app/agents/organigramme')({
	component: OrgChartPage,
})
