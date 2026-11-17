import type { ReactNode } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Field } from '#/components/ui/field'
import { TextInput } from '#/components/ui/text-input'
import { Textarea } from '#/components/ui/textarea'
import {
	IF_MISSED_POLICIES,
	IF_OPEN_POLICIES,
	PRIORITIES,
	type RoutineSettings,
} from '#/domain/routines/types'
import { SelectField } from '#/domain/routines/ui/select-field'
import { cn } from '#/lib/utils'

interface Props {
	settings: RoutineSettings
	agents: ReadonlyArray<string>
	projects: ReadonlyArray<string>
	connectors: ReadonlyArray<string>
	between?: ReactNode
	onChange: (patch: Partial<RoutineSettings>) => void
}

export function RoutineForm({
	settings,
	agents,
	projects,
	connectors,
	between,
	onChange,
}: Props) {
	return (
		<>
			<SectionCard title="Identité">
				<div className="flex max-w-xl flex-col gap-3">
					<Field label="Nom" required>
						<TextInput
							placeholder="Newsletter produit hebdomadaire"
							value={settings.name}
							onChange={(event) => onChange({ name: event.target.value })}
						/>
					</Field>
					<Field label="Consigne" hint="Devient le contenu de la tâche créée">
						<Textarea
							rows={5}
							placeholder="Ce que l'agent doit faire à chaque passage…"
							value={settings.instruction}
							onChange={(event) =>
								onChange({ instruction: event.target.value })
							}
						/>
					</Field>
					<div className="flex flex-col gap-1.5">
						<SelectField
							label="Agent"
							hint="Reçoit la tâche à chaque passage"
							value={settings.agent}
							options={agents}
							onChange={(agent) => onChange({ agent })}
						/>
						<p
							className={cn(
								'text-xs',
								connectors.length > 0
									? 'text-muted-foreground'
									: 'text-signal-attention-ink',
							)}
						>
							{connectors.length > 0
								? `Connecteurs autorisés pour cet agent : ${connectors.join(', ')}`
								: "Cet agent n'a accès à aucun connecteur"}
						</p>
					</div>
					<SelectField
						label="Projet"
						value={settings.project}
						options={projects}
						onChange={(project) => onChange({ project })}
					/>
					<SelectField
						label="Priorité"
						value={settings.priority}
						options={PRIORITIES}
						onChange={(priority) => onChange({ priority })}
					/>
				</div>
			</SectionCard>
			{between}
			<SectionCard title="Comportement">
				<div className="flex max-w-xl flex-col gap-3">
					<Field
						label="Titre de la tâche"
						hint="Variables : {nom}, {date}, {semaine}"
					>
						<TextInput
							value={settings.titleTemplate}
							onChange={(event) =>
								onChange({ titleTemplate: event.target.value })
							}
						/>
					</Field>
					<SelectField
						label="Si la tâche précédente est encore ouverte"
						hint="Évite d'empiler du travail"
						value={settings.ifOpen}
						options={IF_OPEN_POLICIES}
						onChange={(ifOpen) => onChange({ ifOpen })}
					/>
					<SelectField
						label="Si un passage est manqué"
						hint="Serveur arrêté, routine en pause…"
						value={settings.ifMissed}
						options={IF_MISSED_POLICIES}
						onChange={(ifMissed) => onChange({ ifMissed })}
					/>
				</div>
			</SectionCard>
		</>
	)
}
