import { Store } from '@tanstack/store'
import { useSyncExternalStore } from 'react'

export interface TaskComment {
	id: number
	author: string
	text: string
}

export const commentsStore = new Store<Record<string, TaskComment[]>>({})

export function resetComments(): void {
	commentsStore.setState(() => ({}))
}

let counter = 0

export function addComment(
	taskId: string,
	comment: Omit<TaskComment, 'id'>,
): void {
	commentsStore.setState((state) => ({
		...state,
		[taskId]: [...(state[taskId] ?? []), { ...comment, id: ++counter }],
	}))
}

const EMPTY: TaskComment[] = []

export function useComments(taskId: string): TaskComment[] {
	return useSyncExternalStore(
		(onChange) => commentsStore.subscribe(onChange).unsubscribe,
		() => commentsStore.state[taskId] ?? EMPTY,
		() => EMPTY,
	)
}
