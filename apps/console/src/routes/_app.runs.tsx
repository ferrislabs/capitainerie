import { createFileRoute } from '@tanstack/react-router'
import { RunsPage } from '#/domain/runs/feature/runs-page'

export const Route = createFileRoute('/_app/runs')({ component: RunsPage })
