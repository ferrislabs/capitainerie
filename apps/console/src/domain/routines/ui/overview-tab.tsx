import { Link } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Tile } from '#/components/layout/tile'
import {
	formatDateTime,
	IF_MISSED_OPTIONS,
	IF_OPEN_OPTIONS,
	PRIORITY_OPTIONS,
	policyLabel,
	type Routine,
	relativeTime,
	triggersSummary,
} from '#/domain/routines/model'
import { RunBadge } from '#/domain/routines/ui/run-badge'
import { KeyValueRows } from '#/domain/sources/ui/key-value-rows'

interface Props {
	routine: Routine
	agentName: string | undefined
	projectName: string | undefined
	now: number
}

function nextRunValue(routine: Routine, now: number): ReactNode {
	if (routine.archived) return 'Archivée'
	if (!routine.enabled) return 'En pause'
	if (!routine.next_run_at) return '—'
	return (
		<time
			dateTime={routine.next_run_at}
			title={new Date(routine.next_run_at).toLocaleString('fr-FR')}
		>
			{relativeTime(routine.next_run_at, now)}
		</time>
	)
}

export function OverviewTab({ routine, agentName, projectName, now }: Props) {
	const last = routine.last_run
	const { created, skipped, failed } = routine.counts
	const rows: Array<readonly [string, ReactNode]> = [
		[
			'Agent',
			<Link
				key="agent"
				to="/agents/$agentId/{-$tab}"
				params={{ agentId: routine.agent_id }}
				className="text-primary"
			>
				{agentName ?? 'Agent inconnu'}
			</Link>,
		],
		[
			'Projet',
			routine.project_id ? (projectName ?? 'Projet inconnu') : 'Aucun',
		],
		[
			'Priorité',
			PRIORITY_OPTIONS.find((p) => p.value === routine.priority)?.label ??
				routine.priority,
		],
		[
			'Titre de la tâche',
			<span key="title" className="font-mono text-[11.5px]">
				{routine.title_template}
			</span>,
		],
		['Déclencheurs', triggersSummary(routine)],
		['Si déjà ouverte', policyLabel(IF_OPEN_OPTIONS, routine.if_open)],
		['Si manqué', policyLabel(IF_MISSED_OPTIONS, routine.if_missed)],
		['Fuseau', routine.timezone],
		['Écritures sans approbation', routine.allow_writes ? 'Oui' : 'Non'],
		[
			'Slug',
			<span key="slug" className="font-mono text-[11.5px]">
				{routine.slug}
			</span>,
		],
		['Créée le', new Date(routine.created_at).toLocaleDateString('fr-FR')],
	]
	return (
		<div>
			<div className="mb-2.5 grid grid-cols-4 gap-2.5 max-[1100px]:grid-cols-2">
				<Tile
					label="Prochain passage"
					value={nextRunValue(routine, now)}
					hint={
						routine.next_run_at && routine.enabled && !routine.archived
							? formatDateTime(routine.next_run_at)
							: triggersSummary(routine)
					}
				/>
				<Tile
					label="Dernier passage"
					value={
						routine.last_run_at ? (
							<time
								dateTime={routine.last_run_at}
								title={new Date(routine.last_run_at).toLocaleString('fr-FR')}
							>
								{relativeTime(routine.last_run_at, now)}
							</time>
						) : (
							'Jamais'
						)
					}
					hint={last ? <RunBadge run={last} /> : 'aucun passage'}
				/>
				<Tile label="Tâches créées" value={created} hint="au total" />
				<Tile
					label="Ignorés · échecs"
					value={`${skipped} · ${failed}`}
					hint="passages sans tâche"
				/>
			</div>
			<div className="grid grid-cols-2 items-start gap-2.5 max-[1100px]:grid-cols-1">
				<SectionCard title="Consigne" flush>
					<p className="m-0 p-3 whitespace-pre-wrap">{routine.prompt}</p>
				</SectionCard>
				<SectionCard title="Configuration" flush>
					<KeyValueRows rows={rows} />
				</SectionCard>
			</div>
		</div>
	)
}
