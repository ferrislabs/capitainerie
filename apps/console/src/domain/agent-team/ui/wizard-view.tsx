import { Link } from '@tanstack/react-router'
import { Play } from 'lucide-react'
import type { ReactNode } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Checkbox } from '#/components/ui/checkbox'
import { NativeSelect } from '#/components/ui/native-select'
import { TextInput } from '#/components/ui/text-input'
import { Textarea } from '#/components/ui/textarea'
import {
	ADAPTERS,
	DIRECTION,
	EFFORTS,
	MODELS,
	ROLES,
	SKILL_ALL,
	WAKE_INTERVALS,
} from '#/domain/agent-team/constants'
import { AdapterChip } from '#/domain/agent-team/ui/adapter-chip'
import { FieldRow } from '#/domain/agent-team/ui/field-row'
import { Mascot } from '#/domain/agent-team/ui/mascot'
import { Options } from '#/domain/agent-team/ui/options'
import {
	toggled,
	WIZARD_LAST_STEP,
	type WizardState,
} from '#/domain/agent-team/wizard'
import { cn } from '#/lib/utils'

const STEPS = [
	'Nom',
	'Adaptateur',
	'Connexion',
	'Configuration',
	'Confirmation',
] as const

const CONNECTIONS = [
	{ id: 'sub', label: 'Abonnement', hint: 'Compte Claude' },
	{ id: 'key', label: 'Clé API', hint: "Facturation à l'usage" },
	{ id: 'gw', label: 'Passerelle', hint: 'Passerelle perso' },
] as const

const NEXT_LABEL = [
	"Choisir l'adaptateur →",
	"Configurer l'agent →",
	'Connecter →',
	'Continuer →',
] as const

interface Props {
	state: WizardState
	reportsOptions: ReadonlyArray<string>
	sources: ReadonlyArray<{ id: string; name: string }>
	connectors: ReadonlyArray<string>
	canAdvance: boolean
	onChange: (patch: Partial<WizardState>) => void
	onBack: () => void
	onNext: () => void
	onCreate: () => void
}

function ChoiceGroup({
	legend,
	hint,
	children,
}: {
	legend: string
	hint?: string
	children: ReactNode
}) {
	return (
		<fieldset className="flex min-w-0 flex-col gap-1.5">
			<legend className="font-medium">{legend}</legend>
			<div className="flex flex-wrap gap-x-4 gap-y-1.5">{children}</div>
			{hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
		</fieldset>
	)
}

function Recap({ state, sources }: Pick<Props, 'state' | 'sources'>) {
	const conn = CONNECTIONS.find((c) => c.id === state.conn)?.label
	const rows: ReadonlyArray<readonly [string, string]> = [
		['Nom', state.name.trim()],
		['Adaptateur', state.adapter],
		['Connexion', conn ?? ''],
		['Rôle', state.role + (state.title ? ` · ${state.title}` : '')],
		['Rapporte à', state.reports],
		['Modèle', `${state.model} · effort ${state.effort.toLowerCase()}`],
		[
			'Réveils',
			state.interval === 'Manuelle'
				? 'Manuels'
				: `Toutes les ${state.interval}`,
		],
		['Compétences', `${state.skills.length} activées`],
		[
			'Sources',
			state.sources.length
				? sources
						.filter((s) => state.sources.includes(s.id))
						.map((s) => s.name)
						.join(', ')
				: 'Aucune',
		],
		[
			'Connecteurs',
			state.connectors.length
				? state.connectors.join(', ')
				: 'Aucun (accès refusé)',
		],
	]
	return (
		<div className="rounded-lg border bg-background">
			{rows.map(([label, value]) => (
				<FieldRow key={label} label={label}>
					<span>{value}</span>
				</FieldRow>
			))}
		</div>
	)
}

export function WizardView({
	state,
	reportsOptions,
	sources,
	connectors,
	canAdvance,
	onChange,
	onBack,
	onNext,
	onCreate,
}: Props) {
	const { step } = state
	return (
		<div className="mx-auto flex w-full max-w-2xl flex-col gap-4 rounded-lg border bg-card p-5">
			<ol
				aria-label="Étapes de création"
				className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground"
			>
				{STEPS.map((label, index) => (
					<li
						key={label}
						aria-current={index === step ? 'step' : undefined}
						className={cn(
							index === step && 'font-semibold text-foreground',
							index < step && 'text-signal-success',
						)}
					>
						{index < step ? '✓' : index + 1}. {label}
					</li>
				))}
			</ol>

			{step <= 1 ? (
				<div className="flex flex-col items-center gap-1 text-center">
					<Mascot name={state.name} />
					<h2 className="text-lg font-semibold">
						{step === 0
							? 'Rencontre ton prochain agent'
							: 'Choisis un adaptateur'}
					</h2>
					<p className="text-muted-foreground">
						{step === 0
							? 'Commence par un nom. Fais-en le tien.'
							: `Comment ${state.name.trim()} doit-il travailler ?`}
					</p>
				</div>
			) : null}

			{step === 0 ? (
				<TextInput
					aria-label="Nom de l'agent"
					placeholder="ex. Darnold"
					autoComplete="off"
					value={state.name}
					onChange={(event) => onChange({ name: event.target.value })}
				/>
			) : null}

			{step === 1 ? (
				<div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
					{ADAPTERS.map((adapter) => (
						<button
							key={adapter.name}
							type="button"
							aria-pressed={state.adapter === adapter.name}
							onClick={() => onChange({ adapter: adapter.name })}
							className={cn(
								'flex items-center gap-2 rounded-md border px-2.5 py-2 text-left hover:bg-accent',
								state.adapter === adapter.name &&
									'border-primary bg-accent font-medium',
							)}
						>
							<AdapterChip adapter={adapter.name} />
							{adapter.name}
						</button>
					))}
				</div>
			) : null}

			{step === 2 ? (
				<div className="flex flex-col gap-3">
					<h2 className="text-center text-lg font-semibold">
						Connecter un modèle
					</h2>
					<div className="grid grid-cols-3 gap-2">
						{CONNECTIONS.map((option) => (
							<button
								key={option.id}
								type="button"
								aria-pressed={state.conn === option.id}
								onClick={() => onChange({ conn: option.id, key: '' })}
								className={cn(
									'flex flex-col items-center gap-1 rounded-md border px-2 py-3 hover:bg-accent',
									state.conn === option.id && 'border-primary bg-accent',
								)}
							>
								<AdapterChip adapter={state.adapter} />
								<b className="font-semibold">{option.label}</b>
								<small className="text-muted-foreground">{option.hint}</small>
							</button>
						))}
					</div>
					{state.conn === 'sub' ? (
						<p className="text-muted-foreground">
							Une page de connexion s'ouvre dans ton navigateur. Reviens ici
							ensuite, la connexion est détectée toute seule.
						</p>
					) : state.conn === 'key' ? (
						<TextInput
							aria-label="Clé API"
							type="password"
							placeholder="sk-ant-…"
							autoComplete="off"
							value={state.key}
							onChange={(event) => onChange({ key: event.target.value })}
						/>
					) : (
						<TextInput
							aria-label="URL de la passerelle"
							placeholder="https://gateway.exemple.fr/v1"
							value={state.key}
							onChange={(event) => onChange({ key: event.target.value })}
						/>
					)}
				</div>
			) : null}

			{step === 3 ? (
				<div className="flex flex-col gap-3">
					<h2 className="text-center text-lg font-semibold">
						Configurer {state.name.trim()}
					</h2>
					<div className="rounded-lg border bg-background">
						<FieldRow label="Rôle">
							<NativeSelect
								aria-label="Rôle"
								value={state.role}
								onChange={(event) => onChange({ role: event.target.value })}
							>
								<Options values={ROLES} />
							</NativeSelect>
						</FieldRow>
						<FieldRow label="Titre">
							<TextInput
								aria-label="Titre"
								placeholder="ex. Responsable des retours"
								value={state.title}
								onChange={(event) => onChange({ title: event.target.value })}
							/>
						</FieldRow>
						<FieldRow label="Rapporte à">
							<NativeSelect
								aria-label="Rapporte à"
								value={state.reports}
								onChange={(event) => onChange({ reports: event.target.value })}
							>
								<Options values={[DIRECTION, ...reportsOptions]} />
							</NativeSelect>
						</FieldRow>
						<FieldRow label="Modèle">
							<NativeSelect
								aria-label="Modèle"
								value={state.model}
								onChange={(event) => onChange({ model: event.target.value })}
							>
								<Options values={MODELS} />
							</NativeSelect>
						</FieldRow>
						<FieldRow label="Effort de réflexion">
							<NativeSelect
								aria-label="Effort de réflexion"
								value={state.effort}
								onChange={(event) => onChange({ effort: event.target.value })}
							>
								<Options values={EFFORTS} />
							</NativeSelect>
						</FieldRow>
						<FieldRow label="Cadence des réveils">
							<NativeSelect
								aria-label="Cadence des réveils"
								value={state.interval}
								onChange={(event) => onChange({ interval: event.target.value })}
							>
								<Options values={WAKE_INTERVALS} />
							</NativeSelect>
						</FieldRow>
						<FieldRow label="Instructions de départ (AGENTS.md)">
							<Textarea
								aria-label="Instructions de départ"
								rows={4}
								placeholder="Mission, périmètre, règles. Tu pourras tout modifier ensuite."
								value={state.instr}
								onChange={(event) => onChange({ instr: event.target.value })}
							/>
						</FieldRow>
					</div>
					<ChoiceGroup legend="Compétences">
						{SKILL_ALL.map((skill) => (
							<Checkbox
								key={skill}
								label={skill}
								checked={state.skills.includes(skill)}
								onCheckedChange={() =>
									onChange({ skills: toggled(state.skills, skill) })
								}
							/>
						))}
					</ChoiceGroup>
					<ChoiceGroup
						legend="Sources à écouter (optionnel)"
						hint="L'agent se réveille à chaque événement reçu. Les sources se créent depuis la page Sources."
					>
						{sources.map((source) => (
							<Checkbox
								key={source.id}
								label={source.name}
								checked={state.sources.includes(source.id)}
								onCheckedChange={() =>
									onChange({ sources: toggled(state.sources, source.id) })
								}
							/>
						))}
					</ChoiceGroup>
					<ChoiceGroup
						legend="Connecteurs autorisés (aucun par défaut)"
						hint="Sans autorisation explicite, l'agent n'a accès à aucun connecteur. Tu pourras en ajouter ensuite."
					>
						{connectors.map((name) => (
							<Checkbox
								key={name}
								label={name}
								checked={state.connectors.includes(name)}
								onCheckedChange={() =>
									onChange({ connectors: toggled(state.connectors, name) })
								}
							/>
						))}
					</ChoiceGroup>
				</div>
			) : null}

			{step === WIZARD_LAST_STEP ? (
				<div className="flex flex-col gap-3">
					<h2 className="text-center text-lg font-semibold">Tout est prêt</h2>
					<Recap state={state} sources={sources} />
					{state.tested ? (
						<InlineAlert tone="success">
							Test réussi : l'agent a répondu en 2,4 s.
						</InlineAlert>
					) : null}
					<div>
						<Button
							type="button"
							variant="outline"
							onClick={() => onChange({ tested: true })}
						>
							<Play aria-hidden="true" />
							Tester l'agent
						</Button>
					</div>
				</div>
			) : null}

			<div className="flex items-center justify-between">
				{step === 0 ? (
					<Button asChild variant="ghost">
						<Link to="/agents/equipe">Annuler</Link>
					</Button>
				) : (
					<Button type="button" variant="ghost" onClick={onBack}>
						← Retour
					</Button>
				)}
				{step === WIZARD_LAST_STEP ? (
					<Button type="button" onClick={onCreate}>
						Créer l'agent
					</Button>
				) : (
					<Button type="button" disabled={!canAdvance} onClick={onNext}>
						{NEXT_LABEL[step]}
					</Button>
				)}
			</div>
		</div>
	)
}
