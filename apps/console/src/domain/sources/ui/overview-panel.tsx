import { Link } from '@tanstack/react-router'
import { KeyRound } from 'lucide-react'
import type { ReactNode } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Tile } from '#/components/layout/tile'
import { Button } from '#/components/ui/button'
import { relativeAge } from '#/domain/dashboard/format'
import type { IssuedSource, Source } from '#/domain/sources/model'
import { IssuedWebhook } from '#/domain/sources/ui/issued-webhook'
import { KeyValueRows } from '#/domain/sources/ui/key-value-rows'

interface Props {
	source: Source
	agentName: string | undefined
	projectName: string | undefined
	issued: IssuedSource | null
	rotating: boolean
	onRotate: () => void
	onDismissIssued: () => void
}

const yesNo = (value: boolean) => (value ? 'Oui' : 'Non')

export function OverviewPanel({
	source,
	agentName,
	projectName,
	issued,
	rotating,
	onRotate,
	onDismissIssued,
}: Props) {
	const rows: Array<readonly [string, ReactNode]> = [
		['Type', 'Webhook entrant'],
		[
			'Slug',
			<span key="slug" className="font-mono text-[11.5px]">
				{source.slug}
			</span>,
		],
		[
			'Agent par défaut',
			source.default_agent_id ? (
				<Link
					key="agent"
					to="/agents/$agentId/{-$tab}"
					params={{ agentId: source.default_agent_id }}
					className="text-primary"
				>
					{agentName ?? 'Agent inconnu'}
				</Link>
			) : (
				'Aucun : les tâches restent sans assignation'
			),
		],
		['Projet', source.project_id ? (projectName ?? 'Projet inconnu') : 'Aucun'],
		["Lancer l'agent automatiquement", yesNo(source.auto_run)],
		['Écritures sans approbation', yesNo(source.allow_writes)],
		['Créée le', new Date(source.created_at).toLocaleDateString('fr-FR')],
	]
	if (source.description) rows.splice(1, 0, ['Description', source.description])
	return (
		<div>
			<div className="mb-2.5 grid grid-cols-2 gap-2.5">
				<Tile
					label="Événements 24 h"
					value={source.events_last_24h}
					hint="reçus"
				/>
				<Tile
					label="Dernier événement"
					value={
						source.last_event_at ? relativeAge(source.last_event_at) : 'Jamais'
					}
				/>
			</div>
			<SectionCard
				title="Webhook"
				actions={
					source.archived ? null : (
						<Button
							type="button"
							size="sm"
							variant="outline"
							disabled={rotating}
							onClick={onRotate}
						>
							<KeyRound aria-hidden="true" />
							Régénérer le token
						</Button>
					)
				}
			>
				{issued ? (
					<IssuedWebhook issued={issued} onDismiss={onDismissIssued} />
				) : (
					<p className="m-0 text-muted-foreground">
						{source.archived
							? 'Source archivée : son webhook ne répond plus.'
							: 'URL masquée, régénérez le token pour en obtenir une nouvelle.'}
					</p>
				)}
			</SectionCard>
			<SectionCard title="Configuration" flush>
				<KeyValueRows rows={rows} />
			</SectionCard>
		</div>
	)
}
