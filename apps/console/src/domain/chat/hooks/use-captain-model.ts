import { useModelAssignments } from '#/domain/settings/hooks/use-settings-queries'

export type CaptainModel =
	| { kind: 'loading' }
	| { kind: 'unknown' }
	| { kind: 'ready'; model: string }

export function useCaptainModel(): CaptainModel {
	const assignments = useModelAssignments()
	if (assignments.isPending) return { kind: 'loading' }
	const saved = assignments.data?.assignments
	if (!saved) return { kind: 'unknown' }
	return { kind: 'ready', model: (saved.captain ?? saved.writer).model }
}
