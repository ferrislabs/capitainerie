import { useId, useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import {
	useAddTrigger,
	useRemoveTrigger,
	useRotateTriggerToken,
	useUpdateTrigger,
} from '#/domain/routines/hooks/use-routines'
import {
	describeRoutineError,
	type IssuedTrigger,
	type Routine,
	type Trigger,
} from '#/domain/routines/model'
import {
	defaultSchedule,
	scheduleDraftOf,
	scheduleDto,
} from '#/domain/routines/schedule'
import { ConfirmDialog } from '#/domain/routines/ui/confirm-dialog'
import { ScheduleEditor } from '#/domain/routines/ui/schedule-editor'
import { TriggerCard } from '#/domain/routines/ui/trigger-card'
import { IssuedWebhook } from '#/domain/sources/ui/issued-webhook'
import { CTL, Empty } from '#/domain/sources/ui/mockup'
import { RotateTokenDialog } from '#/domain/sources/ui/rotate-token-dialog'
import { cn } from '#/lib/utils'

type Kind = Trigger['kind']

interface Props {
	routine: Routine
	now: number
	notify: (message: string) => void
}

const ROUTINE_HOOK_HINT =
	"Un POST sur cette URL lance un passage de la routine ; le corps de la requête est ignoré. Si la tâche précédente est encore ouverte, la règle de la routine s'applique."

export function TriggersPanel({ routine, now, notify }: Props) {
	const add = useAddTrigger()
	const update = useUpdateTrigger()
	const remove = useRemoveTrigger()
	const rotate = useRotateTriggerToken()
	const [kind, setKind] = useState<Kind>('planning')
	const [editing, setEditing] = useState<string | null>(null)
	const [error, setError] = useState<string | null>(null)
	const [issued, setIssued] = useState<IssuedTrigger | null>(null)
	const [toRotate, setToRotate] = useState<Trigger | null>(null)
	const [toRemove, setToRemove] = useState<Trigger | null>(null)
	const selectId = useId()
	const routineId = routine.id
	const busy =
		add.isPending || update.isPending || remove.isPending || rotate.isPending
	const routineActive = routine.enabled && !routine.archived
	const fail = (failure: unknown) => notify(describeRoutineError(failure))

	const showIssued = (next: IssuedTrigger) => {
		if (next.token && next.webhook_url) setIssued(next)
	}

	const toggle = (trigger: Trigger, enabled: boolean) =>
		update.mutate(
			{
				path: { routine_id: routineId, trigger_id: trigger.id },
				body: { enabled },
			},
			{
				onSuccess: () =>
					notify(enabled ? 'Déclencheur activé' : 'Déclencheur en pause'),
				onError: fail,
			},
		)

	const startAdd = () => {
		setError(null)
		if (kind === 'planning') {
			setEditing('new')
			return
		}
		add.mutate(
			{ path: { routine_id: routineId }, body: { kind: 'webhook' } },
			{
				onSuccess: (next) => {
					showIssued(next)
					notify('Webhook ajouté')
				},
				onError: fail,
			},
		)
	}

	const confirmRotate = () => {
		const trigger = toRotate
		setToRotate(null)
		if (!trigger) return
		rotate.mutate(
			{ path: { routine_id: routineId, trigger_id: trigger.id } },
			{
				onSuccess: (next) => {
					showIssued(next)
					notify('Nouveau token généré : l’ancienne URL ne fonctionne plus')
				},
				onError: fail,
			},
		)
	}

	const confirmRemove = () => {
		const trigger = toRemove
		setToRemove(null)
		if (!trigger) return
		remove.mutate(
			{ path: { routine_id: routineId, trigger_id: trigger.id } },
			{
				onSuccess: () => {
					if (issued?.trigger_id === trigger.id) setIssued(null)
					notify('Déclencheur retiré')
				},
				onError: fail,
			},
		)
	}

	return (
		<div>
			{issued?.token && issued.webhook_url ? (
				<div className="mb-2.5">
					<IssuedWebhook
						issued={{ token: issued.token, webhook_url: issued.webhook_url }}
						hint={ROUTINE_HOOK_HINT}
						onDismiss={() => setIssued(null)}
					/>
				</div>
			) : null}
			{routine.triggers.length === 0 && editing !== 'new' ? (
				<SectionCard title="Déclencheurs" flush>
					<Empty title="Aucun déclencheur">
						La routine ne se lance qu'avec « Lancer maintenant ». Ajoute un
						planning ou un webhook.
					</Empty>
				</SectionCard>
			) : null}
			{routine.triggers.map((trigger) => (
				<TriggerCard
					key={trigger.id}
					trigger={trigger}
					routineActive={routineActive}
					busy={busy}
					now={now}
					editor={
						editing === trigger.id ? (
							<ScheduleEditor
								initial={scheduleDraftOf(trigger.schedule)}
								submitLabel="Enregistrer le planning"
								pending={update.isPending}
								error={error}
								onCancel={() => setEditing(null)}
								onSubmit={(schedule) => {
									setError(null)
									update.mutate(
										{
											path: { routine_id: routineId, trigger_id: trigger.id },
											body: { schedule: scheduleDto(schedule) },
										},
										{
											onSuccess: () => {
												setEditing(null)
												notify('Planning enregistré')
											},
											onError: (failure) =>
												setError(describeRoutineError(failure)),
										},
									)
								}}
							/>
						) : undefined
					}
					onToggle={(enabled) => toggle(trigger, enabled)}
					onEdit={() => {
						setError(null)
						setEditing(trigger.id)
					}}
					onRotate={() => setToRotate(trigger)}
					onRemove={() => setToRemove(trigger)}
				/>
			))}
			{editing === 'new' ? (
				<SectionCard title="Nouveau planning" flush>
					<ScheduleEditor
						initial={defaultSchedule()}
						submitLabel="Ajouter le planning"
						pending={add.isPending}
						error={error}
						onCancel={() => setEditing(null)}
						onSubmit={(schedule) => {
							setError(null)
							add.mutate(
								{
									path: { routine_id: routineId },
									body: { kind: 'planning', schedule: scheduleDto(schedule) },
								},
								{
									onSuccess: () => {
										setEditing(null)
										notify('Planning ajouté')
									},
									onError: (failure) => setError(describeRoutineError(failure)),
								},
							)
						}}
					/>
				</SectionCard>
			) : null}
			<SectionCard title="Lancement manuel" flush>
				<p className="m-0 p-3 text-muted-foreground">
					Toujours disponible : « Lancer maintenant » crée une tâche tout de
					suite, sans attendre un déclencheur ni appliquer les règles de la
					routine.
				</p>
			</SectionCard>
			{routine.archived ? null : (
				<div className="flex gap-1.5">
					<select
						id={selectId}
						aria-label="Type de déclencheur"
						className={cn(CTL, 'max-w-[220px]')}
						value={kind}
						onChange={(event) =>
							setKind(event.target.value === 'webhook' ? 'webhook' : 'planning')
						}
					>
						<option value="planning">Planning</option>
						<option value="webhook">Webhook</option>
					</select>
					<Button
						type="button"
						variant="outline"
						disabled={busy || editing === 'new'}
						onClick={startAdd}
					>
						Ajouter un déclencheur
					</Button>
				</div>
			)}
			<RotateTokenDialog
				open={toRotate !== null}
				name={routine.name}
				onOpenChange={(open) => {
					if (!open) setToRotate(null)
				}}
				onConfirm={confirmRotate}
			/>
			<ConfirmDialog
				open={toRemove !== null}
				title="Retirer ce déclencheur ?"
				description={
					toRemove?.kind === 'webhook'
						? "L'URL de ce webhook cessera de fonctionner immédiatement."
						: 'Ce planning ne créera plus de tâche. Les passages déjà enregistrés restent dans l’historique.'
				}
				confirmLabel="Retirer"
				onOpenChange={(open) => {
					if (!open) setToRemove(null)
				}}
				onConfirm={confirmRemove}
			/>
		</div>
	)
}
