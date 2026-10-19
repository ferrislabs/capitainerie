import { COLUMNS, type Column } from '#/domain/kanban/columns'

export interface DragData {
	column: Column
}

export function columnOfData(data: unknown): Column | undefined {
	if (typeof data !== 'object' || data === null || !('column' in data)) {
		return undefined
	}
	const column = (data as { column: unknown }).column
	return COLUMNS.find((candidate) => candidate === column)
}

export function columnDropId(column: Column): string {
	return `column:${column}`
}
