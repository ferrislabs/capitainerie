import { Play } from 'lucide-react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { Switch } from '#/components/ui/switch'
import { TextInput } from '#/components/ui/text-input'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import {
	ADAPTERS,
	DIRECTION,
	EFFORTS,
	MODELS,
	ROLES,
} from '#/domain/agent-team/constants'
import { FieldRow } from '#/domain/agent-team/ui/field-row'
import { Options } from '#/domain/agent-team/ui/options'

interface Props {
	form: AgentForm
	reportsOptions: ReadonlyArray<string>
	testResult: string | undefined
	onChange: (patch: Partial<AgentForm>) => void
	onTest: () => void
	onChooseConnection: () => void
}

export function RuntimeTab({
	form,
	reportsOptions,
	testResult,
	onChange,
	onTest,
	onChooseConnection,
}: Props) {
	return (
		<div className="flex flex-col gap-4">
			<SectionCard title="Identité de l'agent" flush>
				<FieldRow label="Nom">
					<TextInput
						aria-label="Nom"
						value={form.name}
						onChange={(event) => onChange({ name: event.target.value })}
					/>
				</FieldRow>
				<FieldRow label="Titre">
					<TextInput
						aria-label="Titre"
						value={form.title}
						onChange={(event) => onChange({ title: event.target.value })}
					/>
				</FieldRow>
				<FieldRow label="Rôle">
					<NativeSelect
						aria-label="Rôle"
						value={form.role}
						onChange={(event) => onChange({ role: event.target.value })}
					>
						<Options values={ROLES} />
					</NativeSelect>
				</FieldRow>
				<FieldRow label="Rapporte à">
					<NativeSelect
						aria-label="Rapporte à"
						value={form.reports}
						onChange={(event) => onChange({ reports: event.target.value })}
					>
						<Options values={[DIRECTION, ...reportsOptions]} />
					</NativeSelect>
				</FieldRow>
			</SectionCard>
			<SectionCard title="Adaptateur" flush>
				<FieldRow label="Type d'adaptateur">
					<NativeSelect
						aria-label="Type d'adaptateur"
						value={form.adapter}
						onChange={(event) => onChange({ adapter: event.target.value })}
					>
						<Options values={ADAPTERS.map((a) => a.name)} />
					</NativeSelect>
				</FieldRow>
				<div className="border-t px-3 py-2.5">
					<b className="font-semibold">
						Authentification existante, non gérée par les connexions
					</b>
					<p className="text-muted-foreground">
						Cet agent garde son authentification actuelle jusqu'à ce que tu
						choisisses et testes une connexion gérée. Confirme le compte et les
						personnes autorisées avant de l'adopter.
					</p>
					<Button
						type="button"
						variant="outline"
						size="sm"
						className="mt-2"
						onClick={onChooseConnection}
					>
						Choisir une connexion gérée
					</Button>
				</div>
			</SectionCard>
			<SectionCard title="Modèle" flush>
				<FieldRow label="Modèle">
					<NativeSelect
						aria-label="Modèle"
						value={form.model}
						onChange={(event) => onChange({ model: event.target.value })}
					>
						<Options values={MODELS} />
					</NativeSelect>
				</FieldRow>
				<FieldRow label="Effort de réflexion">
					<NativeSelect
						aria-label="Effort de réflexion"
						value={form.effort}
						onChange={(event) => onChange({ effort: event.target.value })}
					>
						<Options values={EFFORTS} />
					</NativeSelect>
				</FieldRow>
			</SectionCard>
			<SectionCard title="Configuration" flush>
				<FieldRow label="Moteur d'exécution" hint="ACP par défaut">
					<NativeSelect
						aria-label="Moteur d'exécution"
						value={form.engine}
						onChange={(event) => onChange({ engine: event.target.value })}
					>
						<Options values={['ACP', 'Claude CLI']} />
					</NativeSelect>
				</FieldRow>
				<FieldRow label="Activer Chrome" hint="Navigation web pendant les runs">
					<Switch
						aria-label="Activer Chrome"
						checked={form.chrome}
						onCheckedChange={(chrome) => onChange({ chrome })}
					/>
				</FieldRow>
				<FieldRow
					label="Ignorer les permissions"
					hint="À réserver aux environnements isolés"
				>
					<Switch
						aria-label="Ignorer les permissions"
						checked={form.skip}
						onCheckedChange={(skip) => onChange({ skip })}
					/>
				</FieldRow>
				<FieldRow label="Tours max par run">
					<TextInput
						aria-label="Tours max par run"
						type="number"
						min={1}
						placeholder="Illimité"
						value={form.turns}
						onChange={(event) => onChange({ turns: event.target.value })}
					/>
				</FieldRow>
			</SectionCard>
			<SectionCard title="Politique d'exécution" flush>
				<FieldRow label="Réveil sur intervalle" hint="Heartbeat planifié">
					<Switch
						aria-label="Réveil sur intervalle"
						checked={form.hb}
						onCheckedChange={(hb) => onChange({ hb })}
					/>
				</FieldRow>
				<FieldRow label="Intervalle">
					<NativeSelect
						aria-label="Intervalle"
						value={form.interval}
						onChange={(event) => onChange({ interval: event.target.value })}
					>
						<Options values={['15 min', '1 h', '6 h', '24 h']} />
					</NativeSelect>
				</FieldRow>
			</SectionCard>
			<SectionCard title="Tester l'agent">
				<div className="flex items-center gap-3">
					<span className="flex-1 text-muted-foreground">
						Vérifie que le runtime et le modèle répondent.
					</span>
					<Button type="button" variant="outline" onClick={onTest}>
						<Play aria-hidden="true" />
						Lancer le test
					</Button>
				</div>
				{testResult ? (
					<output className="mt-2 block text-signal-success">
						{testResult}
					</output>
				) : null}
			</SectionCard>
		</div>
	)
}
