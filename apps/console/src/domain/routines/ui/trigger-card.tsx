import type { ReactNode } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import {
	formatDateTime,
	relativeTime,
	type Trigger,
} from '#/domain/routines/model'
import { scheduleText } from '#/domain/routines/schedule'
import { Kv, Sev, Sw } from '#/domain/sources/ui/kit'

interface Props {
	trigger: Trigger
	routineActive: boolean
	busy: boolean
	now: number
	/** Éditeur du planning, à la place du résumé. */
	editor?: ReactNode
	onToggle: (enabled: boolean) => void
	onEdit: () => void
	onRotate: () => void
	onRemove: () => void
}

function NextRun({
	trigger,
	routineActive,
	now,
}: {
	trigger: Trigger
	routineActive: boolean
	now: number
}) {
	if (!routineActive) return <>Routine en pause ou archivée</>
	if (!trigger.enabled) return <>Déclencheur en pause</>
	if (!trigger.next_run_at) return <>Aucun passage prévu</>
	return (
		<time
			dateTime={trigger.next_run_at}
			title={new Date(trigger.next_run_at).toLocaleString('fr-FR')}
		>
			{`${formatDateTime(trigger.next_run_at)} (${relativeTime(trigger.next_run_at, now)})`}
		</time>
	)
}

export function TriggerCard({
	trigger,
	routineActive,
	busy,
	now,
	editor,
	onToggle,
	onEdit,
	onRotate,
	onRemove,
}: Props) {
	const planning = trigger.kind === 'planning'
	const title = planning ? 'Planning' : 'Webhook'
	return (
		<SectionCard
			title={title}
			flush
			actions={
				<>
					{planning && trigger.schedule ? (
						<Sev tone="info">{scheduleText(trigger.schedule)}</Sev>
					) : null}
					<Sw
						aria-label={`Activer le ${title.toLowerCase()}`}
						checked={trigger.enabled}
						disabled={busy}
						onCheckedChange={onToggle}
					/>
				</>
			}
		>
			{editor ?? (
				<>
					{planning ? (
						<Kv label="Prochain passage">
							<NextRun
								trigger={trigger}
								routineActive={routineActive}
								now={now}
							/>
						</Kv>
					) : (
						<>
							<Kv label="URL">
								<span className="text-muted-foreground">
									Affichée une seule fois, à la création ou après régénération
									du token.
								</span>
							</Kv>
							<Kv label="Token">
								<span className="font-mono text-[11.5px]">
									{trigger.token_hint
										? `••••••••${trigger.token_hint}`
										: '••••••••'}
								</span>
							</Kv>
							<Kv label="Appel">
								<span className="text-muted-foreground">
									Un POST sur l'URL lance un passage, selon la règle « si la
									tâche précédente est encore ouverte ». Le corps est ignoré.
								</span>
							</Kv>
						</>
					)}
					<div className="flex flex-wrap justify-end gap-2 border-t px-3 py-2">
						{planning ? (
							<Button
								type="button"
								size="sm"
								variant="outline"
								disabled={busy}
								onClick={onEdit}
							>
								Modifier
							</Button>
						) : (
							<Button
								type="button"
								size="sm"
								variant="outline"
								disabled={busy}
								onClick={onRotate}
							>
								Régénérer le token
							</Button>
						)}
						<Button
							type="button"
							size="sm"
							variant="outline"
							disabled={busy}
							onClick={onRemove}
						>
							Retirer
						</Button>
					</div>
				</>
			)}
		</SectionCard>
	)
}
