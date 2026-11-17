import { createFileRoute } from '@tanstack/react-router'
import { AgentWizardPage } from '#/domain/agent-team/feature/agent-wizard-page'

export const Route = createFileRoute('/_app/agents/nouveau')({
	component: AgentWizardPage,
})
