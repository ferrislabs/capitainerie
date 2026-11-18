import { type FormEvent, useId, useMemo, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import {
	IF_MISSED_OPTIONS,
	IF_OPEN_OPTIONS,
	NAME_MAX,
	type PolicyOption,
	PRIORITY_OPTIONS,
	type RoutineDraft,
	SLUG_MAX,
	slugFromName,
	TITLE_TEMPLATE_MAX,
	timezoneOptions,
	validateDraft,
} from '#/domain/routines/model'
import { ScheduleFields } from '#/domain/routines/ui/schedule-fields'
import { SelectField } from '#/domain/routines/ui/select-field'
import { CTL, Fr, Sw } from '#/domain/sources/ui/mockup'
import { cn } from '#/lib/utils'

interface Option {
	id: string
	name: string
}

interface Props {
	mode: 'create' | 'edit'
	initial: RoutineDraft
	agents: ReadonlyArray<Option>
	projects: ReadonlyArray<Option>
	pending: boolean
	error: string | null
	onSubmit: (draft: RoutineDraft) => void
	onCancel?: () => void
}

const MONO = 'font-mono text-[11.5px]'

function PolicyField<T extends string>({
	label,
	value,
	options,
	onChange,
}: {
	label: string
	value: T
	options: ReadonlyArray<PolicyOption<T>>
	onChange: (value: T) => void
}) {
	const help = options.find((o) => o.value === value)?.help
	return (
		<SelectField
			label={label}
			value={value}
			options={options}
			onChange={onChange}
			hint={help}
		/>
	)
}

export function RoutineForm({
	mode,
	initial,
	agents,
	projects,
	pending,
	error,
	onSubmit,
	onCancel,
}: Props) {
	const [draft, setDraft] = useState<RoutineDraft>(initial)
	const [slugTouched, setSlugTouched] = useState(mode === 'edit')
	const [invalid, setInvalid] = useState<string | null>(null)
	const ids = useId()
	const patch = (change: Partial<RoutineDraft>) =>
		setDraft((current) => ({ ...current, ...change }))
	const timezones = useMemo(
		() => timezoneOptions(draft.timezone).map((z) => ({ value: z, label: z })),
		[draft.timezone],
	)
	const knownAgent =
		!draft.agentId || agents.some((a) => a.id === draft.agentId)
	const knownProject =
		!draft.projectId || projects.some((p) => p.id === draft.projectId)

	const submit = (event: FormEvent) => {
		event.preventDefault()
		const problem = validateDraft(draft)
		setInvalid(problem)
		if (!problem) onSubmit(draft)
	}

	return (
		<form
			className="flex w-full max-w-[880px] flex-col gap-4"
			aria-label={mode === 'create' ? 'Nouvelle routine' : 'Réglages'}
			onSubmit={submit}
			noValidate
		>
			<SectionCard title="Identité" flush className="mb-0">
				<Fr label={<label htmlFor={`${ids}-name`}>Nom</label>}>
					<input
						id={`${ids}-name`}
						className={CTL}
						maxLength={NAME_MAX}
						placeholder="Newsletter produit hebdomadaire"
						value={draft.name}
						onChange={(e) => {
							const name = e.target.value
							patch(slugTouched ? { name } : { name, slug: slugFromName(name) })
						}}
					/>
				</Fr>
				<Fr
					label={<label htmlFor={`${ids}-slug`}>Slug</label>}
					hint={
						mode === 'create'
							? 'Déduit du nom. Minuscules, chiffres et tirets.'
							: 'Minuscules, chiffres et tirets.'
					}
				>
					<input
						id={`${ids}-slug`}
						className={cn(CTL, MONO)}
						maxLength={SLUG_MAX}
						placeholder="newsletter-produit"
						value={draft.slug}
						onChange={(e) => {
							setSlugTouched(true)
							patch({ slug: e.target.value })
						}}
					/>
				</Fr>
				<Fr
					label={<label htmlFor={`${ids}-prompt`}>Consigne</label>}
					hint="Devient la description de la tâche créée à chaque passage."
				>
					<textarea
						id={`${ids}-prompt`}
						className={cn(CTL, 'block resize-y leading-normal')}
						rows={5}
						placeholder="Ce que l'agent doit faire à chaque passage…"
						value={draft.prompt}
						onChange={(e) => patch({ prompt: e.target.value })}
					/>
				</Fr>
				{mode === 'create' ? (
					<Fr
						label="Active"
						hint="En pause, aucun déclencheur ne crée de tâche."
					>
						<Sw
							aria-label="Active"
							checked={draft.enabled}
							onCheckedChange={(enabled) => patch({ enabled })}
						/>
					</Fr>
				) : null}
			</SectionCard>
			<SectionCard title="Qui reçoit le travail" flush className="mb-0">
				<Fr
					label={<label htmlFor={`${ids}-agent`}>Agent</label>}
					hint="Chaque passage crée une tâche assignée à cet agent et lance son exécution."
				>
					<select
						id={`${ids}-agent`}
						className={CTL}
						value={draft.agentId}
						onChange={(e) => patch({ agentId: e.target.value })}
					>
						<option value="">Choisir un agent…</option>
						{knownAgent ? null : (
							<option value={draft.agentId}>Agent inconnu</option>
						)}
						{agents.map((a) => (
							<option key={a.id} value={a.id}>
								{a.name}
							</option>
						))}
					</select>
				</Fr>
				<Fr
					label={<label htmlFor={`${ids}-project`}>Projet</label>}
					hint="Les tâches créées sont rangées dans ce projet."
				>
					<select
						id={`${ids}-project`}
						className={CTL}
						value={draft.projectId}
						onChange={(e) => patch({ projectId: e.target.value })}
					>
						<option value="">Aucun projet</option>
						{knownProject ? null : (
							<option value={draft.projectId}>Projet inconnu</option>
						)}
						{projects.map((p) => (
							<option key={p.id} value={p.id}>
								{p.name}
							</option>
						))}
					</select>
				</Fr>
				<SelectField
					label="Priorité"
					value={draft.priority}
					options={PRIORITY_OPTIONS}
					onChange={(priority) => patch({ priority })}
				/>
				<Fr
					label={<label htmlFor={`${ids}-title`}>Titre de la tâche</label>}
					hint="Variables : {nom} (nom de la routine), {date} (jj/mm/aaaa)."
				>
					<input
						id={`${ids}-title`}
						className={cn(CTL, MONO)}
						maxLength={TITLE_TEMPLATE_MAX}
						value={draft.titleTemplate}
						onChange={(e) => patch({ titleTemplate: e.target.value })}
					/>
				</Fr>
				<Fr
					label="Écritures sans approbation"
					hint="Autorise l'agent à écrire via ses connecteurs sans demander d'approbation, pour les exécutions lancées par cette routine."
				>
					<Sw
						aria-label="Écritures sans approbation"
						checked={draft.allowWrites}
						onCheckedChange={(allowWrites) => patch({ allowWrites })}
					/>
				</Fr>
			</SectionCard>
			<SectionCard title="Planification" flush className="mb-0">
				{mode === 'create' ? (
					<Fr
						label="Planifier"
						hint="Sans planning, la routine se lance à la main ou par un webhook, à ajouter ensuite."
					>
						<Sw
							aria-label="Planifier"
							checked={draft.planned}
							onCheckedChange={(planned) => patch({ planned })}
						/>
					</Fr>
				) : null}
				{mode === 'create' && draft.planned ? (
					<ScheduleFields
						schedule={draft.schedule}
						onChange={(change) =>
							patch({ schedule: { ...draft.schedule, ...change } })
						}
					/>
				) : null}
				<SelectField
					label="Fuseau horaire"
					value={draft.timezone}
					options={timezones}
					onChange={(timezone) => patch({ timezone })}
					hint={
						mode === 'edit'
							? "Les heures des plannings s'entendent dans ce fuseau. Les plannings se modifient dans l'onglet Déclencheurs."
							: "Les heures des plannings s'entendent dans ce fuseau."
					}
				/>
			</SectionCard>
			<SectionCard title="Comportement" flush className="mb-0">
				<PolicyField
					label="Si la tâche précédente est encore ouverte"
					value={draft.ifOpen}
					options={IF_OPEN_OPTIONS}
					onChange={(ifOpen) => patch({ ifOpen })}
				/>
				<PolicyField
					label="Si un passage est manqué"
					value={draft.ifMissed}
					options={IF_MISSED_OPTIONS}
					onChange={(ifMissed) => patch({ ifMissed })}
				/>
				<p className="m-0 border-t px-3 py-2 text-[12px] text-muted-foreground">
					« Lancer maintenant » crée toujours une tâche, sans tenir compte de
					ces règles.
				</p>
			</SectionCard>
			{invalid || error ? (
				<InlineAlert tone="danger">{invalid ?? error}</InlineAlert>
			) : null}
			<div className="flex justify-end gap-2">
				<Button
					type="button"
					variant="outline"
					onClick={() => {
						setDraft(initial)
						setInvalid(null)
						onCancel?.()
					}}
				>
					Annuler
				</Button>
				<Button type="submit" disabled={pending}>
					{pending
						? 'Enregistrement…'
						: mode === 'create'
							? 'Créer la routine'
							: 'Enregistrer'}
				</Button>
			</div>
		</form>
	)
}
