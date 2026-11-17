import { SectionCard } from '#/components/layout/section-card'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import { CTL, FieldRow, Note } from '#/domain/agent-team/ui/field-row'
import { Options } from '#/domain/agent-team/ui/options'
import { Toggle } from '#/domain/agent-team/ui/toggle'

interface Props {
	form: AgentForm
	onChange: (patch: Partial<AgentForm>) => void
}

export function PermsTab({ form, onChange }: Props) {
	return (
		<>
			<SectionCard title="Profil de confiance" flush>
				<FieldRow
					label="Profil"
					hint="Collaboration normale : défaut du travail courant"
				>
					<select
						aria-label="Profil"
						className={CTL}
						value={form.trust}
						onChange={(event) => onChange({ trust: event.target.value })}
					>
						<Options
							values={['Lecture seule', 'Collaboration normale', 'Avancé']}
						/>
					</select>
				</FieldRow>
				<Note>
					Les permissions avancées restent modifiables par l'extension de
					permissions quand elle est installée.
				</Note>
			</SectionCard>
			<SectionCard title="Permissions" flush>
				<FieldRow
					label="Peut créer des agents"
					hint="Accorde aussi le droit d'assigner des tâches"
				>
					<Toggle
						label="Créer des agents"
						checked={form.pCreate}
						onChange={(pCreate) => onChange({ pCreate })}
					/>
				</FieldRow>
				<FieldRow
					label="Peut créer ou importer des skills"
					hint="Installer, importer, créer et scanner les skills de l'organisation"
				>
					<Toggle
						label="Importer des skills"
						checked={form.pSkills}
						onChange={(pSkills) => onChange({ pSkills })}
					/>
				</FieldRow>
				<FieldRow
					label="Peut assigner des tâches"
					hint="Accordé par une permission explicite de l'organisation"
				>
					<Toggle
						label="Assigner des tâches"
						checked={form.pAssign}
						onChange={(pAssign) => onChange({ pAssign })}
					/>
				</FieldRow>
			</SectionCard>
		</>
	)
}
