import { createFileRoute } from '@tanstack/react-router'
import { TasksPage } from '#/domain/tasks/feature/tasks-page'
import { parseTasksSearch } from '#/domain/tasks/task-search'

export const Route = createFileRoute('/_app/tasks/')({
	validateSearch: parseTasksSearch,
	component: TasksRoute,
})

function TasksRoute() {
	return <TasksPage search={Route.useSearch()} />
}
