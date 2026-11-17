import { createFileRoute } from '@tanstack/react-router'
import { TaskPage } from '#/domain/tasks/feature/task-page'

export const Route = createFileRoute('/_app/tasks/$taskId')({
	component: TaskRoute,
})

function TaskRoute() {
	const { taskId } = Route.useParams()
	return <TaskPage pathId={taskId} />
}
