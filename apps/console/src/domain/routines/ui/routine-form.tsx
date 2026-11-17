import { Link } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import {
	IF_MISSED_POLICIES,
	IF_OPEN_POLICIES,
	PRIORITIES,
	type RoutineSettings,
} from '#/domain/routines/types'
import { SelectField } from '#/domain/routines/ui/select-field'
import { CTL, Fr, Sev } from '#/domain/sources/ui/mockup'
import { cn } from '#/lib/utils'

interface Props {
	settings: RoutineSettings
	agents: ReadonlyArray<string>
	projects: ReadonlyArray<string>
	connectors: ReadonlyArray<string>
	agentId?: string | undefined
	between?: ReactNode
	onChange: (patch: Partial<RoutineSettings>) => void
}

export function RoutineForm({
	settings,
	agents,
	projects,
	connectors,
	agentId,
	between,
	onChange,
}: Props) {
	return (
		<>
			<SectionCard title="Identité" flush>
				<Fr label={<label htmlFor="routine-name">Nom</label>}>
					<input
						id="routine-name"
						className={CTL}
						placeholder="Newsletter produit hebdomadaire"
						value={settings.name}
						onChange={(event) => onChange({ name: event.target.value })}
					/>
				</Fr>
				<Fr
					label={<label htmlFor="routine-instruction">Consigne</label>}
					hint="Devient le contenu de la tâche créée"
				>
					<textarea
						id="routine-instruction"
						className={cn(CTL, 'block resize-y leading-normal')}
						rows={5}
						placeholder="Ce que l'agent doit faire à chaque passage…"
						value={settings.instruction}
						onChange={(event) => onChange({ instruction: event.target.value })}
					/>
				</Fr>
				<SelectField
					label="Agent"
					hint="Reçoit la tâche à chaque passage"
					value={settings.agent}
					options={agents}
					onChange={(agent) => onChange({ agent })}
				/>
				<Fr label="">
					<span className="text-[12px] text-muted-foreground">
						{connectors.length > 0 ? (
							`Connecteurs autorisés pour cet agent : ${connectors.join(', ')}`
						) : agentId ? (
							<Sev tone="warn">Cet agent n'a accès à aucun connecteur</Sev>
						) : (
							'Agent hors liste'
						)}{' '}
						{agentId ? (
							<>
								·{' '}
								<Link
									to="/agents/$agentId/{-$tab}"
									params={{ agentId, tab: 'connectors' }}
									className="text-primary"
								>
									Gérer
								</Link>
							</>
						) : null}
					</span>
				</Fr>
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
			</SectionCard>
			{between}
			<SectionCard title="Comportement" flush>
				<Fr
					label={<label htmlFor="routine-title">Titre de la tâche</label>}
					hint="Variables : {nom}, {date}, {semaine}"
				>
					<input
						id="routine-title"
						className={CTL}
						value={settings.titleTemplate}
						onChange={(event) =>
							onChange({ titleTemplate: event.target.value })
						}
					/>
				</Fr>
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
			</SectionCard>
		</>
	)
}
