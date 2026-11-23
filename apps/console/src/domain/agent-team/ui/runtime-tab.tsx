import { Link } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import { DIRECTION, EFFORTS, ROLES } from '#/domain/agent-team/constants'
import { type Harness, harnessOfAgent } from '#/domain/agent-team/harness'
import { CTL, FieldRow } from '#/domain/agent-team/ui/field-row'
import { type OptionItem, Options } from '#/domain/agent-team/ui/options'
import { DetailSection } from '#/domain/agent-team/ui/panel'
import { Toggle } from '#/domain/agent-team/ui/toggle'

interface Props {
	form: AgentForm
	harnesses: ReadonlyArray<Harness>
	reportsOptions: ReadonlyArray<OptionItem>
	onChange: (patch: Partial<AgentForm>) => void
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
	harnesses,
	reportsOptions,
	onChange,
}: Props): ReactNode {
	const current = harnessOfAgent(harnesses, form.model, form.adapter)
	const models = current?.models ?? []
	const modelValues: ReadonlyArray<OptionItem> = models.includes(form.model)
		? models.map((m) => ({ value: m, label: m }))
		: [
				{ value: form.model, label: form.model || 'Modèle par défaut' },
				...models.map((m) => ({ value: m, label: m })),
			]
	return (
		<>
			<DetailSection title="Identité de l'agent" flush>
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
			</DetailSection>
			<DetailSection title="Adaptateur" flush>
				<Select
					label="Adaptateur"
					value={current?.name ?? form.adapter}
					values={harnesses.map((h) => h.name)}
					onChange={(adapter) => {
						const next = harnesses.find((h) => h.name === adapter)
						onChange(
							next ? { adapter, model: next.default_model } : { adapter },
						)
					}}
				/>
			</DetailSection>
			<DetailSection title="Modèle" flush>
				<Select
					label="Modèle"
					value={form.model}
					values={modelValues}
					onChange={(model) => onChange({ model })}
				/>
				<p className="m-0 border-t px-6 py-4 text-muted-foreground">
					L'adaptateur est décidé par le modèle : choisir un modèle d'un autre
					fournisseur change l'adaptateur qui exécute l'agent. La clé utilisée
					est votre clé personnelle du fournisseur, sinon celle de
					l'organisation.{' '}
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
			</DetailSection>
			<DetailSection title="Configuration" flush>
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
			</DetailSection>
			<DetailSection title="Politique d'exécution" flush>
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
			</DetailSection>
		</>
	)
}
