import { createFileRoute } from '@tanstack/react-router'
import { AgentsPage } from '#/domain/agents/pages/feature/agents-page'

export const Route = createFileRoute('/_app/agents')({ component: AgentsPage })
