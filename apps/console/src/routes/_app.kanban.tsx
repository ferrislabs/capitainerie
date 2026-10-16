import { createFileRoute } from '@tanstack/react-router'
import { KanbanPage } from '#/domain/kanban/feature/kanban-page'

export const Route = createFileRoute('/_app/kanban')({ component: KanbanPage })
