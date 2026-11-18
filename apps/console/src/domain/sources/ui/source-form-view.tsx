import { type FormEvent, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import {
	SLUG_MAX,
	type SourceDraft,
	slugFromName,
	validateDraft,
} from '#/domain/sources/model'
import { CTL, Fr, Sw } from '#/domain/sources/ui/kit'
import { cn } from '#/lib/utils'

interface Option {
	id: string
	name: string
}

interface Props {
	mode: 'create' | 'edit'
	initial: SourceDraft
	agents: ReadonlyArray<Option>
	projects: ReadonlyArray<Option>
	pending: boolean
	error: string | null
	onSubmit: (draft: SourceDraft) => void
	onCancel?: () => void
}

const MONO = 'font-mono text-[11.5px]'

function Toggle({
	label,
	checked,
	disabled,
	onChange,
}: {
	label: string
	checked: boolean
	disabled?: boolean
	onChange: (checked: boolean) => void
}) {
	return (
		<Sw
			aria-label={label}
			checked={checked}
			disabled={disabled}
			onCheckedChange={onChange}
		/>
	)
}

export function SourceFormView({
	mode,
	initial,
	agents,
	projects,
	pending,
	error,
	onSubmit,
	onCancel,
}: Props) {
	const [draft, setDraft] = useState<SourceDraft>(initial)
	const [slugTouched, setSlugTouched] = useState(mode === 'edit')
	const [invalid, setInvalid] = useState<string | null>(null)
	const patch = (change: Partial<SourceDraft>) =>
		setDraft((current) => ({ ...current, ...change }))
	const knownAgent =
		!draft.defaultAgentId || agents.some((a) => a.id === draft.defaultAgentId)
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
			aria-label={mode === 'create' ? 'Nouvelle source' : 'Paramètres'}
			onSubmit={submit}
		>
			<SectionCard title="Webhook entrant" flush>
				<Fr label="Nom">
					<input
						className={CTL}
						aria-label="Nom"
						placeholder="Pièces reçues sur Chatwoot"
						value={draft.name}
						onChange={(e) => {
							const name = e.target.value
							patch(slugTouched ? { name } : { name, slug: slugFromName(name) })
						}}
					/>
				</Fr>
				<Fr
					label="Slug"
					hint={
						mode === 'create'
							? 'Déduit du nom. Minuscules, chiffres et tirets.'
							: 'Minuscules, chiffres et tirets.'
					}
				>
					<input
						className={cn(CTL, MONO)}
						aria-label="Slug"
						maxLength={SLUG_MAX}
						placeholder="pieces-chatwoot"
						value={draft.slug}
						onChange={(e) => {
							setSlugTouched(true)
							patch({ slug: e.target.value })
						}}
					/>
				</Fr>
				<Fr label="Description">
					<input
						className={CTL}
						aria-label="Description"
						value={draft.description}
						onChange={(e) => patch({ description: e.target.value })}
					/>
				</Fr>
				<Fr label="Active" hint="En pause, le webhook refuse les événements.">
					<Toggle
						label="Active"
						checked={draft.enabled}
						onChange={(enabled) => patch({ enabled })}
					/>
				</Fr>
			</SectionCard>
			<SectionCard title="Qui reçoit le travail" flush>
				<Fr
					label="Agent par défaut"
					hint="Chaque événement crée une tâche assignée à cet agent."
				>
					<select
						className={CTL}
						aria-label="Agent par défaut"
						value={draft.defaultAgentId}
						onChange={(e) => {
							const defaultAgentId = e.target.value
							patch(
								defaultAgentId
									? { defaultAgentId }
									: { defaultAgentId, autoRun: false, allowWrites: false },
							)
						}}
					>
						<option value="">Aucun agent</option>
						{knownAgent ? null : (
							<option value={draft.defaultAgentId}>Agent inconnu</option>
						)}
						{agents.map((a) => (
							<option key={a.id} value={a.id}>
								{a.name}
							</option>
						))}
					</select>
				</Fr>
				<Fr
					label="Projet"
					hint="Les tâches créées sont rangées dans ce projet."
				>
					<select
						className={CTL}
						aria-label="Projet"
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
				<Fr
					label="Lancer l'agent"
					hint={
						draft.defaultAgentId
							? "L'agent démarre dès que la tâche est créée."
							: 'Choisis d’abord un agent par défaut.'
					}
				>
					<Toggle
						label="Lancer l'agent automatiquement"
						checked={draft.autoRun}
						disabled={!draft.defaultAgentId}
						onChange={(autoRun) =>
							patch(autoRun ? { autoRun } : { autoRun, allowWrites: false })
						}
					/>
				</Fr>
				<Fr
					label="Écritures sans approbation"
					hint="Autorise l'agent à écrire via ses connecteurs sans demander d'approbation, pour les exécutions lancées par cette source."
				>
					<Toggle
						label="Écritures sans approbation"
						checked={draft.allowWrites}
						disabled={!draft.autoRun}
						onChange={(allowWrites) => patch({ allowWrites })}
					/>
				</Fr>
			</SectionCard>
			{invalid || error ? (
				<InlineAlert tone="danger">{invalid ?? error}</InlineAlert>
			) : null}
			<div className="flex justify-end gap-2">
				{onCancel ? (
					<Button type="button" variant="outline" onClick={onCancel}>
						Annuler
					</Button>
				) : null}
				<Button type="submit" disabled={pending}>
					{pending
						? 'Enregistrement…'
						: mode === 'create'
							? 'Créer la source'
							: 'Enregistrer'}
				</Button>
			</div>
		</form>
	)
}
