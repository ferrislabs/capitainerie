import type { Source, SourceEvent } from '#/domain/sources/model'

export function source(id: string, extra: Partial<Source> = {}): Source {
	return {
		id,
		slug: id,
		name: `Source ${id}`,
		kind: 'webhook',
		description: '',
		enabled: true,
		config: {},
		default_agent_id: null,
		project_id: null,
		auto_run: false,
		allow_writes: false,
		archived: false,
		events_last_24h: 0,
		last_event_at: null,
		created_by: 'u',
		created_at: '2026-11-01T00:00:00Z',
		updated_at: '2026-11-01T00:00:00Z',
		...extra,
	}
}

export function sourceEvent(
	id: string,
	extra: Partial<SourceEvent> = {},
): SourceEvent {
	return {
		id,
		source_id: 's1',
		received_at: '2026-11-18T10:00:00Z',
		status: 'received',
		summary: `Événement ${id}`,
		card_id: null,
		run_id: null,
		error: null,
		...extra,
	}
}
