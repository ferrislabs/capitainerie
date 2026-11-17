import type { DemoTask } from '#/domain/demo'

interface Input {
	title: string
	depth: number
	openChildren: number
	maxDepth: number
	maxParallel: number
}

export type DelegationResult = { ok: true } | { ok: false; reason: string }

export function openChildCount(children: ReadonlyArray<DemoTask>): number {
	return children.filter((c) => c.status !== 'done' && c.status !== 'cancelled')
		.length
}

export function canDelegate(input: Input): DelegationResult {
	if (!input.title.trim()) {
		return { ok: false, reason: 'Écris le titre de la sous-tâche' }
	}
	if (input.depth + 1 > input.maxDepth) {
		return {
			ok: false,
			reason: `Profondeur maximale atteinte (${input.maxDepth})`,
		}
	}
	if (input.openChildren >= input.maxParallel) {
		return {
			ok: false,
			reason: `Limite atteinte : ${input.maxParallel} sous-tâches en parallèle`,
		}
	}
	return { ok: true }
}
