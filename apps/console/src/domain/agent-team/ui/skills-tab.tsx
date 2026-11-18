import { type FormEvent, useState } from 'react'
import { Button } from '#/components/ui/button'
import { SKILL_ALL } from '#/domain/agent-team/constants'
import { CTL } from '#/domain/agent-team/ui/field-row'
import { HumanAvatar } from '#/domain/agent-team/ui/human-avatar'
import { Panel } from '#/domain/agent-team/ui/panel'
import { Empty, ROW, Rows, Ttl } from '#/domain/agent-team/ui/rows'

interface Props {
	skills: ReadonlyArray<string>
	busy: boolean
	onAdd: (skill: string) => void
	onRemove: (skill: string) => void
}

export function SkillsTab({ skills, busy, onAdd, onRemove }: Props) {
	const [draft, setDraft] = useState('')
	const suggestions = SKILL_ALL.filter((skill) => !skills.includes(skill))
	const submit = (event: FormEvent) => {
		event.preventDefault()
		const skill = draft.trim()
		if (!skill || skills.includes(skill)) return
		onAdd(skill)
		setDraft('')
	}
	return (
		<>
			<p className="mb-2 mt-0 text-muted-foreground">
				<b className="font-semibold text-foreground">
					{skills.length} {skills.length > 1 ? 'activées' : 'activée'}
				</b>{' '}
				· appliqué au prochain run
			</p>
			<Panel>
				{skills.length === 0 ? (
					<Empty title="Aucune compétence">
						Ajoute une compétence par son nom ci-dessous.
					</Empty>
				) : (
					<Rows>
						{skills.map((skill) => (
							<div key={skill} className={ROW}>
								<HumanAvatar
									initials={skill[0]?.toUpperCase() ?? ''}
									size={26}
								/>
								<Ttl title={skill} titleMono />
								<Button
									type="button"
									variant="outline"
									size="sm"
									disabled={busy}
									aria-label={`Retirer ${skill}`}
									onClick={() => onRemove(skill)}
								>
									Retirer
								</Button>
							</div>
						))}
					</Rows>
				)}
				<form
					aria-label="Ajouter une compétence"
					onSubmit={submit}
					className="flex gap-1.5 border-t px-3 py-3"
				>
					<input
						aria-label="Nom de la compétence"
						className={CTL}
						placeholder="ex. humanizer"
						value={draft}
						onChange={(event) => setDraft(event.target.value)}
					/>
					<Button type="submit" disabled={busy || !draft.trim()}>
						Ajouter
					</Button>
				</form>
				{suggestions.length > 0 ? (
					<div className="flex flex-wrap items-center gap-1.5 border-t px-3 py-2.5 text-xs text-muted-foreground">
						Suggestions :
						{suggestions.map((skill) => (
							<button
								key={skill}
								type="button"
								disabled={busy}
								aria-label={`Ajouter ${skill}`}
								onClick={() => onAdd(skill)}
								className="rounded-md bg-secondary px-2 py-0.5 font-mono text-[11.5px] text-muted-foreground hover:text-foreground"
							>
								+ {skill}
							</button>
						))}
					</div>
				) : null}
			</Panel>
		</>
	)
}
