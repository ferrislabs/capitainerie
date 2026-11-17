import { SectionCard } from '#/components/layout/section-card'
import { NativeSelect } from '#/components/ui/native-select'
import { Switch } from '#/components/ui/switch'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import { FieldRow } from '#/domain/agent-team/ui/field-row'
import { Options } from '#/domain/agent-team/ui/options'

interface Props {
	form: AgentForm
	onChange: (patch: Partial<AgentForm>) => void
}

export function PermsTab({ form, onChange }: Props) {
	return (
		<div className="flex flex-col gap-4">
			<SectionCard title="Profil de confiance" flush>
				<FieldRow
					label="Profil"
					hint="Collaboration normale : défaut du travail courant"
				>
					<NativeSelect
						aria-label="Profil"
						value={form.trust}
						onChange={(event) => onChange({ trust: event.target.value })}
					>
						<Options
							values={['Lecture seule', 'Collaboration normale', 'Avancé']}
						/>
					</NativeSelect>
				</FieldRow>
				<p className="border-t px-3 py-2.5 text-muted-foreground">
					Les permissions avancées restent modifiables par l'extension de
					permissions quand elle est installée.
				</p>
			</SectionCard>
			<SectionCard title="Permissions" flush>
				<FieldRow
					label="Peut créer des agents"
					hint="Accorde aussi le droit d'assigner des tâches"
				>
					<Switch
						aria-label="Créer des agents"
						checked={form.pCreate}
						onCheckedChange={(pCreate) => onChange({ pCreate })}
					/>
				</FieldRow>
				<FieldRow
					label="Peut créer ou importer des skills"
					hint="Installer, importer, créer et scanner les skills de l'organisation"
				>
					<Switch
						aria-label="Importer des skills"
						checked={form.pSkills}
						onCheckedChange={(pSkills) => onChange({ pSkills })}
					/>
				</FieldRow>
				<FieldRow
					label="Peut assigner des tâches"
					hint="Accordé par une permission explicite de l'organisation"
				>
					<Switch
						aria-label="Assigner des tâches"
						checked={form.pAssign}
						onCheckedChange={(pAssign) => onChange({ pAssign })}
					/>
				</FieldRow>
			</SectionCard>
		</div>
	)
}
