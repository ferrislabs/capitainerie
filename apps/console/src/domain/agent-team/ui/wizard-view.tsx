import { Link } from '@tanstack/react-router'
import { cloneElement, type ReactElement, type ReactNode, useId } from 'react'
import { Button } from '#/components/ui/button'
import { Combobox } from '#/components/ui/combobox'
import {
	ADAPTERS,
	DIRECTION,
	EFFORTS,
	ROLES,
	WAKE_INTERVALS,
} from '#/domain/agent-team/constants'
import {
	isValidModelId,
	MODEL_ID_ERROR,
	MODEL_ID_PLACEHOLDER,
} from '#/domain/agent-team/model-id'
import { AdapterChip } from '#/domain/agent-team/ui/adapter-chip'
import { CTL } from '#/domain/agent-team/ui/field-row'
import { Mascot } from '#/domain/agent-team/ui/mascot'
import { type OptionItem, Options } from '#/domain/agent-team/ui/options'
import {
	toggled,
	WIZARD_LAST_STEP,
	type WizardState,
} from '#/domain/agent-team/wizard'
import type { RepositoriesState } from '#/domain/integrations/hooks/use-github'
import { cn } from '#/lib/utils'

const STEPS = ['Connexion', 'Configuration', 'Confirmation'] as const

const CONNECTIONS = [
	{ id: 'sub', label: 'Abonnement', hint: 'Compte Claude' },
	{ id: 'key', label: 'Clé API', hint: "Facturation à l'usage" },
	{ id: 'gw', label: 'Avancé', hint: 'Passerelle perso' },
] as const

const CONNECTION_RECAP = {
	sub: 'Abonnement',
	key: 'Clé API',
	gw: 'Passerelle',
} as const

const NEXT_LABEL = [
	"Choisir l'adaptateur →",
	"Configurer l'agent →",
	'Connecter →',
	'Continuer →',
] as const

const CHOICE = 'rounded-[9px] border bg-background hover:border-faint'
const CHOICE_ON =
	'border-primary bg-[color-mix(in_srgb,var(--primary)_10%,var(--background))] hover:border-primary'

interface Props {
	state: WizardState
	reportsOptions: ReadonlyArray<OptionItem>
	connectors: ReadonlyArray<{ slug: string; name: string }>
	connectorsLoading: boolean
	repositories: RepositoriesState
	/** Slugs proposés : catalogue des compétences et sélection courante. */
	skillOptions: ReadonlyArray<string>
	canAdvance: boolean
	onChange: (patch: Partial<WizardState>) => void
	onBack: () => void
	onNext: () => void
	onCreate: () => void
}

function Pills({
	legend,
	hint,
	children,
}: {
	legend: string
	hint?: string
	children: ReactNode
}) {
	return (
		<div className="col-span-full grid gap-1 text-xs text-muted-foreground">
			{legend}
			<div className="flex flex-wrap gap-1.5">{children}</div>
			{hint ? <span className="font-normal">{hint}</span> : null}
		</div>
	)
}

function Pill({
	label,
	checked,
	onChange,
}: {
	label: string
	checked: boolean
	onChange: () => void
}) {
	return (
		<label className="inline-flex items-center gap-1.5 rounded-full border bg-background px-[9px] py-[3px] text-xs text-foreground">
			<input type="checkbox" checked={checked} onChange={onChange} />
			{label}
		</label>
	)
}

function Field({
	label,
	children,
}: {
	label: string
	children: ReactElement<{ id?: string }>
}) {
	const id = useId()
	return (
		<div className="grid gap-1 text-xs text-muted-foreground">
			<label htmlFor={id}>{label}</label>
			{cloneElement(children, { id })}
		</div>
	)
}

function Recap({
	state,
	connectors,
	reportsOptions,
}: Pick<Props, 'state' | 'connectors' | 'reportsOptions'>) {
	const rows: ReadonlyArray<readonly [string, string]> = [
		['Nom', state.name.trim()],
		['Adaptateur', state.adapter],
		['Connexion', CONNECTION_RECAP[state.conn]],
		['Rôle', state.role + (state.title ? ` · ${state.title}` : '')],
		[
			'Rapporte à',
			reportsOptions.find((o) => o.value === state.reports)?.label ?? DIRECTION,
		],
		[
			'Modèle',
			`${state.model.trim() || 'modèle par défaut du fournisseur'} · effort ${state.effort.toLowerCase()}`,
		],
		[
			'Réveils',
			state.interval === 'Manuelle'
				? 'Manuels'
				: `Toutes les ${state.interval}`,
		],
		['Compétences', `${state.skills.length} activées`],
		[
			'Connecteurs',
			state.connectors.length
				? state.connectors
						.map(
							(slug) => connectors.find((c) => c.slug === slug)?.name ?? slug,
						)
						.join(', ')
				: 'Aucun (accès refusé)',
		],
		[
			'Dépôts GitHub',
			state.repositories.length ? state.repositories.join(', ') : 'Aucun',
		],
	]
	return (
		<div className="overflow-hidden rounded-lg border bg-background">
			{rows.map(([label, value]) => (
				<div
					key={label}
					className="grid min-w-0 grid-cols-[180px_minmax(0,1fr)] items-center gap-2.5 border-t px-3 py-2 first:border-t-0"
				>
					<span className="text-muted-foreground">{label}</span>
					<span>{value}</span>
				</div>
			))}
		</div>
	)
}

function RepositoryPicker({
	state,
	value,
	onChange,
}: {
	state: RepositoriesState
	value: ReadonlyArray<string>
	onChange: (next: string[]) => void
}) {
	const id = useId()
	return (
		<div className="col-span-full grid gap-1 text-xs text-muted-foreground">
			<label htmlFor={id}>Dépôts GitHub autorisés (aucun par défaut)</label>
			{state.kind === 'not_connected' ? (
				<span className="font-normal">
					GitHub n'est pas connecté à l'organisation.{' '}
					<Link to="/settings/integrations/github" className="text-primary">
						Connecter GitHub
					</Link>
				</span>
			) : state.kind === 'error' ? (
				<span className="font-normal text-signal-danger">
					Impossible de charger les dépôts GitHub.
				</span>
			) : (
				<Combobox
					id={id}
					multiple
					className="font-mono text-xs"
					items={(state.kind === 'ready' ? state.repositories : []).map(
						(repository) => ({
							value: repository.full_name,
							label: repository.full_name,
						}),
					)}
					value={value}
					loading={state.kind === 'loading'}
					onValueChange={onChange}
					placeholder="Choisir des dépôts…"
					emptyText="Aucun dépôt ne correspond"
				/>
			)}
			<span className="font-normal">
				L'agent pourra auditer ces dépôts ou y proposer des changements en pull
				request brouillon.
			</span>
		</div>
	)
}

function Footer({ children }: { children: ReactNode }) {
	return (
		<div className="mt-[18px] -mx-[22px] flex items-center justify-between border-t px-[22px] py-3">
			{children}
		</div>
	)
}

export function WizardView({
	state,
	reportsOptions,
	connectors,
	connectorsLoading,
	repositories,
	skillOptions,
	canAdvance,
	onChange,
	onBack,
	onNext,
	onCreate,
}: Props) {
	const { step } = state
	const name = state.name.trim()

	const backButton =
		step === 0 ? (
			<Button asChild variant="ghost">
				<Link to="/agents">Annuler</Link>
			</Button>
		) : (
			<Button type="button" variant="ghost" onClick={onBack}>
				← Retour
			</Button>
		)
	const nextButton =
		step === WIZARD_LAST_STEP ? (
			<Button type="button" onClick={onCreate}>
				Créer l'agent
			</Button>
		) : (
			<Button type="button" disabled={!canAdvance} onClick={onNext}>
				{NEXT_LABEL[step]}
			</Button>
		)
	const footer = (
		<Footer>
			{backButton}
			{nextButton}
		</Footer>
	)

	if (step <= 1) {
		return (
			<div className="mx-auto mt-[2vh] w-full max-w-[760px]">
				<div className="rounded-xl border bg-card px-[22px] pt-4 pb-0">
					<ol
						aria-label="Étapes de création"
						className="m-0 flex list-none items-center gap-2 p-0 text-xs text-faint"
					>
						<li
							aria-current={step === 0 ? 'step' : undefined}
							className={cn(step === 0 && 'font-semibold text-foreground')}
						>
							1. Nom
						</li>
						<li aria-hidden="true">›</li>
						<li
							aria-current={step === 1 ? 'step' : undefined}
							className={cn(step === 1 && 'font-semibold text-foreground')}
						>
							2. Adaptateur
						</li>
					</ol>
					<div className="flex flex-col items-center pt-1.5 pb-3.5 text-center">
						<Mascot name={state.name} state="idle" />
						<h1 className="leading-[1.4] mt-1.5 mb-0.5 text-2xl font-bold tracking-[-0.02em]">
							{step === 0
								? 'Rencontre ton prochain agent'
								: 'Choisis un adaptateur'}
						</h1>
						<p className="m-0 text-muted-foreground">
							{step === 0
								? 'Commence par un nom. Fais-en le tien.'
								: `Comment ${name} doit-il travailler ?`}
						</p>
					</div>
					{step === 0 ? (
						<>
							<label
								htmlFor="w-name"
								className="mt-2 mb-1.5 block font-semibold"
							>
								Nom de l'agent
							</label>
							<input
								id="w-name"
								placeholder="ex. Darnold"
								autoComplete="off"
								value={state.name}
								onChange={(event) => onChange({ name: event.target.value })}
								className="h-[38px] w-full rounded-lg border bg-background px-3 text-sm text-foreground focus:border-primary focus:outline-0"
							/>
						</>
					) : (
						<div className="mt-3 grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-2">
							{ADAPTERS.map((adapter) => (
								<button
									key={adapter.name}
									type="button"
									aria-pressed={state.adapter === adapter.name}
									onClick={() => onChange({ adapter: adapter.name })}
									className={cn(
										CHOICE,
										'flex flex-col items-center gap-[7px] px-2 py-3.5 font-medium',
										state.adapter === adapter.name && CHOICE_ON,
									)}
								>
									<AdapterChip adapter={adapter.name} />
									{adapter.name}
								</button>
							))}
						</div>
					)}
					{footer}
				</div>
			</div>
		)
	}

	return (
		<div className="mx-auto mt-[2vh] grid w-full max-w-[880px] grid-cols-[200px_minmax(0,1fr)] gap-7">
			<aside>
				<div className="mb-[18px] flex items-center gap-3">
					<Mascot
						name={state.name}
						state="idle"
						className="h-[72px] w-[85px] shrink-0"
					/>
					<div>
						<b className="block text-lg">{state.name}</b>
						<span className="flex items-center gap-1.5 text-xs text-muted-foreground">
							<AdapterChip adapter={state.adapter} />
							{state.adapter}
						</span>
					</div>
				</div>
				<ol aria-label="Étapes de création" className="m-0 list-none p-0">
					{STEPS.map((label, index) => {
						const at = index + 2
						return (
							<li
								key={label}
								aria-current={at === step ? 'step' : undefined}
								className={cn(
									'flex h-9 items-center gap-2.5 rounded-lg px-2.5 text-faint',
									at === step && 'bg-secondary font-medium text-foreground',
								)}
							>
								<i
									className={cn(
										'grid size-5 place-items-center rounded-full border text-[11px] font-medium not-italic',
										at === step &&
											'border-foreground bg-foreground text-background',
										at < step &&
											'border-signal-success bg-signal-success text-white',
									)}
								>
									{at < step ? '✓' : index + 1}
								</i>
								{label}
							</li>
						)
					})}
				</ol>
			</aside>
			<div className="rounded-xl border bg-card px-[22px] pt-4 pb-0">
				{step === 2 ? (
					<>
						<h1 className="leading-[1.4] mt-1.5 mb-1 text-center text-[22px] font-bold">
							Connecter un modèle
						</h1>
						<div className="my-3 grid grid-cols-3 gap-2">
							{CONNECTIONS.map((option) => (
								<button
									key={option.id}
									type="button"
									aria-pressed={state.conn === option.id}
									onClick={() => onChange({ conn: option.id, key: '' })}
									className={cn(
										CHOICE,
										'flex flex-col items-center gap-[3px] px-1.5 py-3',
										state.conn === option.id && CHOICE_ON,
									)}
								>
									<AdapterChip adapter={state.adapter} />
									<b className="font-semibold">{option.label}</b>
									<small className="text-muted-foreground">{option.hint}</small>
								</button>
							))}
						</div>
						{state.conn === 'sub' ? (
							<p className="m-0 text-muted-foreground">
								Une page de connexion s'ouvre dans ton navigateur. Reviens ici
								ensuite, la connexion est détectée toute seule.
							</p>
						) : state.conn === 'key' ? (
							<Field label="Clé API">
								<input
									className={CTL}
									type="password"
									placeholder="sk-ant-…"
									autoComplete="off"
									value={state.key}
									onChange={(event) => onChange({ key: event.target.value })}
								/>
							</Field>
						) : (
							<Field label="URL de la passerelle">
								<input
									className={CTL}
									placeholder="https://gateway.exemple.fr/v1"
									value={state.key}
									onChange={(event) => onChange({ key: event.target.value })}
								/>
							</Field>
						)}
					</>
				) : null}

				{step === 3 ? (
					<>
						<h1 className="leading-[1.4] mt-1.5 mb-0.5 text-center text-[22px] font-bold">
							Configurer {name}
						</h1>
						<div className="mt-3 grid grid-cols-2 gap-x-3.5 gap-y-2.5">
							<Field label="Rôle">
								<select
									className={CTL}
									value={state.role}
									onChange={(event) => onChange({ role: event.target.value })}
								>
									<Options values={ROLES} />
								</select>
							</Field>
							<Field label="Titre">
								<input
									className={CTL}
									placeholder="ex. Responsable des retours"
									value={state.title}
									onChange={(event) => onChange({ title: event.target.value })}
								/>
							</Field>
							<Field label="Rapporte à">
								<select
									className={CTL}
									value={state.reports}
									onChange={(event) =>
										onChange({ reports: event.target.value })
									}
								>
									<Options
										values={[
											{ value: DIRECTION, label: DIRECTION },
											...reportsOptions,
										]}
									/>
								</select>
							</Field>
							<Field label="Modèle">
								<input
									className={CTL}
									value={state.model}
									placeholder={MODEL_ID_PLACEHOLDER}
									aria-invalid={!isValidModelId(state.model)}
									onChange={(event) => onChange({ model: event.target.value })}
								/>
							</Field>
							{isValidModelId(state.model) ? null : (
								<p role="alert" className="m-0 text-xs text-signal-danger">
									{MODEL_ID_ERROR}
								</p>
							)}
							<Field label="Effort de réflexion">
								<select
									className={CTL}
									value={state.effort}
									onChange={(event) => onChange({ effort: event.target.value })}
								>
									<Options values={EFFORTS} />
								</select>
							</Field>
							<Field label="Cadence des réveils">
								<select
									className={CTL}
									value={state.interval}
									onChange={(event) =>
										onChange({ interval: event.target.value })
									}
								>
									<Options values={WAKE_INTERVALS} />
								</select>
							</Field>
							<label className="col-span-full grid gap-1 text-xs text-muted-foreground">
								Instructions de départ (AGENTS.md)
								<textarea
									className={CTL}
									rows={4}
									placeholder="Mission, périmètre, règles. Tu pourras tout modifier ensuite."
									value={state.instr}
									onChange={(event) => onChange({ instr: event.target.value })}
								/>
							</label>
							<Pills legend="Compétences">
								{skillOptions.map((skill) => (
									<Pill
										key={skill}
										label={skill}
										checked={state.skills.includes(skill)}
										onChange={() =>
											onChange({ skills: toggled(state.skills, skill) })
										}
									/>
								))}
							</Pills>
							<Pills
								legend="Connecteurs autorisés (aucun par défaut)"
								hint="Sans autorisation explicite, l'agent n'a accès à aucun connecteur. Tu pourras en ajouter ensuite."
							>
								{connectorsLoading ? (
									<span className="font-normal">
										Chargement des connecteurs…
									</span>
								) : connectors.length === 0 ? (
									<span className="font-normal">
										Aucun connecteur MCP déclaré pour l'instant.
									</span>
								) : (
									connectors.map((connector) => (
										<Pill
											key={connector.slug}
											label={connector.name}
											checked={state.connectors.includes(connector.slug)}
											onChange={() =>
												onChange({
													connectors: toggled(state.connectors, connector.slug),
												})
											}
										/>
									))
								)}
							</Pills>
							<RepositoryPicker
								state={repositories}
								value={state.repositories}
								onChange={(next) => onChange({ repositories: next })}
							/>
						</div>
					</>
				) : null}

				{step === WIZARD_LAST_STEP ? (
					<>
						<h1 className="leading-[1.4] mt-1.5 mb-2.5 text-center text-[22px] font-bold">
							Tout est prêt
						</h1>
						<Recap
							state={state}
							connectors={connectors}
							reportsOptions={reportsOptions}
						/>
					</>
				) : null}
				{footer}
			</div>
		</div>
	)
}
