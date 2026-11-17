import { useId, useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { Switch } from '#/components/ui/switch'
import { maskedSecret } from '#/domain/routines/routine-draft'
import { type Schedule, scheduleText } from '#/domain/routines/schedule'
import type {
	PlanningTrigger,
	Trigger,
	WebhookTrigger,
} from '#/domain/routines/types'
import { CRON_NOTE, NextRuns } from '#/domain/routines/ui/next-runs'
import { ScheduleFields } from '#/domain/routines/ui/schedule-fields'

export type NewTriggerKind = 'planning' | 'webhook'

interface Props {
	triggers: ReadonlyArray<Trigger>
	nextRunsOf: (schedule: Schedule) => ReadonlyArray<Date>
	onPatchSchedule: (triggerId: string, patch: Partial<Schedule>) => void
	onToggle: (triggerId: string, enabled: boolean) => void
	onRemove: (triggerId: string) => void
	onCopy: (trigger: WebhookTrigger) => void
	onRegenerate: (triggerId: string) => void
	onAdd: (kind: NewTriggerKind) => void
}

interface PlanningProps {
	trigger: PlanningTrigger
	upcoming: ReadonlyArray<Date>
	onPatch: (patch: Partial<Schedule>) => void
	onToggle: (enabled: boolean) => void
	onRemove: () => void
}

function PlanningSection({
	trigger,
	upcoming,
	onPatch,
	onToggle,
	onRemove,
}: PlanningProps) {
	return (
		<SectionCard
			title="Planning"
			actions={
				<>
					<Badge variant="neutral">{scheduleText(trigger.schedule)}</Badge>
					<Button type="button" size="sm" variant="outline" onClick={onRemove}>
						Retirer
					</Button>
				</>
			}
		>
			<div className="flex flex-col gap-3">
				<ScheduleFields schedule={trigger.schedule} onChange={onPatch} />
				<Switch
					label="Activer le planning"
					checked={trigger.enabled}
					onCheckedChange={onToggle}
				/>
				<div className="flex flex-col gap-1.5">
					<h3 className="text-xs text-muted-foreground">Prochains passages</h3>
					<NextRuns runs={upcoming} empty={CRON_NOTE} />
				</div>
			</div>
		</SectionCard>
	)
}

export function TriggersTab({
	triggers,
	nextRunsOf,
	onPatchSchedule,
	onToggle,
	onRemove,
	onCopy,
	onRegenerate,
	onAdd,
}: Props) {
	const [kind, setKind] = useState<NewTriggerKind>('planning')
	const selectId = useId()
	return (
		<div className="flex flex-col gap-4">
			{triggers.map((trigger) =>
				trigger.kind === 'planning' ? (
					<PlanningSection
						key={trigger.id}
						trigger={trigger}
						upcoming={nextRunsOf(trigger.schedule)}
						onPatch={(patch) => onPatchSchedule(trigger.id, patch)}
						onToggle={(enabled) => onToggle(trigger.id, enabled)}
						onRemove={() => onRemove(trigger.id)}
					/>
				) : (
					<SectionCard
						key={trigger.id}
						title="Webhook"
						actions={
							<Button
								type="button"
								size="sm"
								variant="outline"
								onClick={() => onRemove(trigger.id)}
							>
								Retirer
							</Button>
						}
					>
						<div className="flex flex-col gap-3 text-[13px]">
							<div className="flex flex-wrap items-center gap-2">
								<span className="text-muted-foreground">URL</span>
								<span className="font-mono [overflow-wrap:anywhere]">
									{trigger.url}
								</span>
								<Button
									type="button"
									size="sm"
									variant="outline"
									onClick={() => onCopy(trigger)}
								>
									Copier
								</Button>
							</div>
							<p className="text-xs text-muted-foreground">
								POST avec l'en-tête X-Signature
							</p>
							<div className="flex flex-wrap items-center gap-2">
								<span className="text-muted-foreground">Secret</span>
								<span className="font-mono">
									{maskedSecret(trigger.secret)}
								</span>
								<Button
									type="button"
									size="sm"
									variant="outline"
									onClick={() => onRegenerate(trigger.id)}
								>
									Régénérer
								</Button>
							</div>
							<Switch
								label="Activer le webhook"
								checked={trigger.enabled}
								onCheckedChange={(enabled) => onToggle(trigger.id, enabled)}
							/>
						</div>
					</SectionCard>
				),
			)}
			<SectionCard title="Lancement manuel">
				<p className="text-[13px] text-muted-foreground">
					Toujours disponible : « Lancer maintenant » crée une tâche sans
					attendre le planning.
				</p>
			</SectionCard>
			<div className="flex flex-wrap items-center gap-2">
				<label htmlFor={selectId} className="sr-only">
					Type de déclencheur
				</label>
				<NativeSelect
					id={selectId}
					className="max-w-[220px]"
					value={kind}
					onChange={(event) =>
						setKind(event.target.value === 'webhook' ? 'webhook' : 'planning')
					}
				>
					<option value="planning">Planning</option>
					<option value="webhook">Webhook</option>
				</NativeSelect>
				<Button type="button" variant="outline" onClick={() => onAdd(kind)}>
					Ajouter un déclencheur
				</Button>
			</div>
		</div>
	)
}
