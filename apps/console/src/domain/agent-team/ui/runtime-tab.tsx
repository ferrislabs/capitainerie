import { Link } from '@tanstack/react-router'
import { Play } from 'lucide-react'
import type { ReactNode } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import {
	ADAPTERS,
	DIRECTION,
	EFFORTS,
	ROLES,
} from '#/domain/agent-team/constants'
import {
	isValidModelId,
	MODEL_ID_ERROR,
	MODEL_ID_PLACEHOLDER,
} from '#/domain/agent-team/model-id'
import { CTL, FieldRow } from '#/domain/agent-team/ui/field-row'
import { type OptionItem, Options } from '#/domain/agent-team/ui/options'
import { Toggle } from '#/domain/agent-team/ui/toggle'

interface Props {
	form: AgentForm
	reportsOptions: ReadonlyArray<OptionItem>
	testResult: string | undefined
	onChange: (patch: Partial<AgentForm>) => void
	onTest: () => void
	onChooseConnection: () => void
	onAdvanced: () => void
}

function Select({
	label,
	hint,
	value,
	values,
	onChange,
}: {
	label: string
	hint?: string
	value: string
	values: ReadonlyArray<string | OptionItem>
	onChange: (value: string) => void
}) {
	return (
		<FieldRow label={label} hint={hint}>
			<select
				aria-label={label}
				className={CTL}
				value={value}
				onChange={(event) => onChange(event.target.value)}
			>
				<Options values={values} />
			</select>
		</FieldRow>
	)
}

function Text({
	label,
	value,
	placeholder,
	type,
	onChange,
}: {
	label: string
	value: string
	placeholder?: string
	type?: string
	onChange: (value: string) => void
}) {
	return (
		<FieldRow label={label}>
			<input
				aria-label={label}
				className={CTL}
				type={type}
				min={type === 'number' ? 1 : undefined}
				placeholder={placeholder}
				value={value}
				onChange={(event) => onChange(event.target.value)}
			/>
		</FieldRow>
	)
}

function Switched({
	label,
	hint,
	checked,
	onChange,
}: {
	label: string
	hint: string
	checked: boolean
	onChange: (checked: boolean) => void
}) {
	return (
		<FieldRow label={label} hint={hint}>
			<Toggle label={label} checked={checked} onChange={onChange} />
		</FieldRow>
	)
}

export function RuntimeTab({
	form,
	reportsOptions,
	testResult,
	onChange,
	onTest,
	onChooseConnection,
	onAdvanced,
}: Props): ReactNode {
	return (
		<>
			<SectionCard title="Identité de l'agent" flush>
				<Text
					label="Nom"
					value={form.name}
					onChange={(name) => onChange({ name })}
				/>
				<Text
					label="Description"
					value={form.description}
					onChange={(description) => onChange({ description })}
				/>
				<Text
					label="Titre"
					value={form.title}
					onChange={(title) => onChange({ title })}
				/>
				<Select
					label="Rôle"
					value={form.role}
					values={ROLES.includes(form.role) ? ROLES : [form.role, ...ROLES]}
					onChange={(role) => onChange({ role })}
				/>
				<Select
					label="Rapporte à"
					value={form.reports}
					values={[{ value: DIRECTION, label: DIRECTION }, ...reportsOptions]}
					onChange={(reports) => onChange({ reports })}
				/>
			</SectionCard>
			<SectionCard title="Adaptateur" flush>
				<Select
					label="Type d'adaptateur"
					value={form.adapter}
					values={ADAPTERS.map((a) => a.name)}
					onChange={(adapter) => onChange({ adapter })}
				/>
				<div className="border-t px-3 py-2">
					<b className="font-semibold">
						Authentification existante, non gérée par les connexions
					</b>
					<div className="text-muted-foreground">
						Cet agent garde son authentification actuelle jusqu'à ce que tu
						choisisses et testes une connexion gérée. Confirme le compte et les
						personnes autorisées avant de l'adopter.
					</div>
					<p className="mt-2 mb-0">
						<Button
							type="button"
							variant="outline"
							size="sm"
							onClick={onChooseConnection}
						>
							Choisir une connexion gérée
						</Button>
					</p>
				</div>
			</SectionCard>
			<SectionCard title="Modèle" flush>
				<Text
					label="Modèle"
					placeholder={MODEL_ID_PLACEHOLDER}
					value={form.model}
					onChange={(model) => onChange({ model })}
				/>
				{isValidModelId(form.model) ? null : (
					<p role="alert" className="m-0 px-3 pb-2 text-signal-danger">
						{MODEL_ID_ERROR}
					</p>
				)}
				<p className="m-0 border-t px-3 py-2 text-muted-foreground">
					Le modèle utilisé est celui de votre fournisseur par défaut, sauf si
					vous renseignez un identifiant de modèle valide ici.{' '}
					<Link to="/providers" className="text-primary underline">
						Gérer mes fournisseurs
					</Link>
				</p>
				<Select
					label="Effort de réflexion"
					value={form.effort}
					values={EFFORTS}
					onChange={(effort) => onChange({ effort })}
				/>
			</SectionCard>
			<SectionCard title="Configuration" flush>
				<Select
					label="Moteur d'exécution"
					hint="ACP par défaut"
					value={form.engine}
					values={['ACP', 'Claude CLI']}
					onChange={(engine) => onChange({ engine })}
				/>
				<Switched
					label="Activer Chrome"
					hint="Navigation web pendant les runs"
					checked={form.chrome}
					onChange={(chrome) => onChange({ chrome })}
				/>
				<Switched
					label="Ignorer les permissions"
					hint="À réserver aux environnements isolés"
					checked={form.skip}
					onChange={(skip) => onChange({ skip })}
				/>
				<Text
					label="Tours max par run"
					type="number"
					placeholder="Illimité"
					value={form.turns}
					onChange={(turns) => onChange({ turns })}
				/>
			</SectionCard>
			<SectionCard title="Politique d'exécution" flush>
				<Switched
					label="Réveil sur intervalle"
					hint="Heartbeat planifié"
					checked={form.hb}
					onChange={(hb) => onChange({ hb })}
				/>
				<Select
					label="Intervalle"
					value={form.interval}
					values={['15 min', '1 h', '6 h', '24 h']}
					onChange={(interval) => onChange({ interval })}
				/>
				<FieldRow label="Avancé">
					<button
						type="button"
						className="text-xs text-primary"
						onClick={onAdvanced}
					>
						Politique d'exécution avancée
					</button>
				</FieldRow>
			</SectionCard>
			<SectionCard title="Tester l'agent" flush>
				<div className="grid grid-cols-[1fr_auto] items-center gap-2.5 px-3 py-2">
					<span className="text-muted-foreground">
						Vérifie que le runtime et le modèle répondent.
					</span>
					<Button type="button" variant="outline" onClick={onTest}>
						<Play aria-hidden="true" />
						Lancer le test
					</Button>
				</div>
				{testResult ? (
					<output className="mx-3 mb-3 block rounded-[7px] border border-signal-success/45 bg-signal-success/10 px-2.5 py-2 text-signal-success">
						{testResult}
					</output>
				) : null}
			</SectionCard>
		</>
	)
}
