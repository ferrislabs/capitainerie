import { createFileRoute } from '@tanstack/react-router'
import { KanbanPage } from '#/domain/kanban/pages/feature/kanban-page'

export const Route = createFileRoute('/_app/kanban')({ component: KanbanPage })
