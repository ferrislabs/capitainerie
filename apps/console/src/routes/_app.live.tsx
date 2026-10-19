import { createFileRoute } from '@tanstack/react-router'
import { LivePage } from '#/domain/runs/feature/live-page'

export const Route = createFileRoute('/_app/live')({ component: LivePage })
