import { Copy } from 'lucide-react'
import { useId, useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { maskedSecret } from '#/domain/routines/routine-draft'
import { type Schedule, scheduleText } from '#/domain/routines/schedule'
import type {
	PlanningTrigger,
	Trigger,
	WebhookTrigger,
} from '#/domain/routines/types'
import { CRON_NOTE, NextRuns } from '#/domain/routines/ui/next-runs'
import { ScheduleFields } from '#/domain/routines/ui/schedule-fields'
import { CTL, Fr, Sev, Sw } from '#/domain/sources/ui/mockup'
import { cn } from '#/lib/utils'

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
			flush
			actions={
				<>
					<Sev tone="info">{scheduleText(trigger.schedule)}</Sev>
					<Button type="button" size="sm" variant="outline" onClick={onRemove}>
						Retirer
					</Button>
				</>
			}
		>
			<ScheduleFields schedule={trigger.schedule} onChange={onPatch} />
			<Fr label="Actif">
				<Sw
					aria-label="Activer le planning"
					checked={trigger.enabled}
					onCheckedChange={onToggle}
				/>
			</Fr>
			<div className="px-3 pb-1 text-muted-foreground">Prochains passages</div>
			<NextRuns runs={upcoming} empty={CRON_NOTE} />
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
		<div>
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
						flush
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
						<Fr label="URL" hint="POST avec l'en-tête X-Signature">
							<span className="font-mono text-[11.5px] [overflow-wrap:anywhere]">
								{trigger.url}
							</span>{' '}
							<Button
								type="button"
								size="sm"
								variant="outline"
								onClick={() => onCopy(trigger)}
							>
								<Copy aria-hidden="true" />
								Copier
							</Button>
						</Fr>
						<Fr label="Secret">
							<span className="font-mono text-[11.5px]">
								{maskedSecret(trigger.secret)}
							</span>{' '}
							<Button
								type="button"
								size="sm"
								variant="outline"
								onClick={() => onRegenerate(trigger.id)}
							>
								Régénérer
							</Button>
						</Fr>
						<Fr label="Actif">
							<Sw
								aria-label="Activer le webhook"
								checked={trigger.enabled}
								onCheckedChange={(enabled) => onToggle(trigger.id, enabled)}
							/>
						</Fr>
					</SectionCard>
				),
			)}
			<SectionCard title="Lancement manuel" flush>
				<p className="m-0 p-3 text-muted-foreground">
					Toujours disponible : « Lancer maintenant » crée une tâche sans
					attendre le planning.
				</p>
			</SectionCard>
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
				<Button type="button" variant="outline" onClick={() => onAdd(kind)}>
					Ajouter un déclencheur
				</Button>
			</div>
		</div>
	)
}
