import type { Routine, RoutineRun, Trigger } from '#/domain/routines/model'

export function routineRun(
	id: string,
	extra: Partial<RoutineRun> = {},
): RoutineRun {
	return {
		id,
		routine_id: 'r1',
		trigger_id: null,
		origin: 'planning',
		scheduled_for: '2026-11-17T06:00:00Z',
		started_at: '2026-11-17T06:00:01Z',
		status: 'created',
		reason: null,
		card_id: null,
		card_column: null,
		run_id: null,
		run_status: null,
		run_started_at: null,
		run_finished_at: null,
		...extra,
	}
}

export function trigger(id: string, extra: Partial<Trigger> = {}): Trigger {
	return {
		id,
		kind: 'planning',
		enabled: true,
		schedule: { frequency: 'weekly', time: '06:30', days: ['mon'] },
		next_run_at: '2026-11-23T05:30:00Z',
		token_hint: null,
		...extra,
	}
}

export function routine(id: string, extra: Partial<Routine> = {}): Routine {
	return {
		id,
		slug: id,
		name: `Routine ${id}`,
		prompt: 'Faire le point',
		agent_id: 'a-dev',
		project_id: null,
		priority: 'normal',
		title_template: '{nom} · {date}',
		timezone: 'Europe/Paris',
		enabled: true,
		if_open: 'skip',
		if_missed: 'run_once',
		allow_writes: false,
		archived: false,
		next_run_at: null,
		last_run_at: null,
		last_run: null,
		counts: { created: 0, skipped: 0, failed: 0 },
		triggers: [],
		created_by: 'u',
		created_at: '2026-11-01T00:00:00Z',
		updated_at: '2026-11-01T00:00:00Z',
		...extra,
	}
}
