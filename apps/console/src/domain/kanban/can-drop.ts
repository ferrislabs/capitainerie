import type { Column } from '#/domain/kanban/columns'

export function canDrop(from: Column, to: Column): boolean {
	return from !== to
}
