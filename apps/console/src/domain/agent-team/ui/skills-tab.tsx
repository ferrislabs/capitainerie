import { type FormEvent, useState } from 'react'
import { Button } from '#/components/ui/button'
import { CTL } from '#/domain/agent-team/ui/field-row'
import { HumanAvatar } from '#/domain/agent-team/ui/human-avatar'
import { DetailSection } from '#/domain/agent-team/ui/panel'
import { DETAIL_ROW, Empty, Rows, Ttl } from '#/domain/agent-team/ui/rows'

interface Props {
	skills: ReadonlyArray<string>
	/** Slugs du catalogue de compétences (non archivées). */
	catalog: ReadonlyArray<string>
	busy: boolean
	onAdd: (skill: string) => void
	onRemove: (skill: string) => void
}

export function SkillsTab({ skills, catalog, busy, onAdd, onRemove }: Props) {
	const [draft, setDraft] = useState('')
	const suggestions = catalog.filter((skill) => !skills.includes(skill))
	const submit = (event: FormEvent) => {
		event.preventDefault()
		const skill = draft.trim()
		if (!skill || skills.includes(skill)) return
		onAdd(skill)
		setDraft('')
	}
	return (
		<DetailSection
			title="Compétences"
			flush
			description={
				<>
					<b className="font-semibold text-foreground">
						{skills.length} {skills.length > 1 ? 'activées' : 'activée'}
					</b>{' '}
					· appliqué au prochain run
				</>
			}
		>
			{skills.length === 0 ? (
				<Empty title="Aucune compétence">
					Ajoute une compétence par son nom ci-dessous.
				</Empty>
			) : (
				<Rows>
					{skills.map((skill) => (
						<div key={skill} className={DETAIL_ROW}>
							<HumanAvatar initials={skill[0]?.toUpperCase() ?? ''} size={26} />
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
				className="flex gap-2 border-t px-6 py-5"
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
				<div className="flex flex-wrap items-center gap-2 border-t px-6 py-4 text-[13px] text-muted-foreground">
					Suggestions :
					{suggestions.map((skill) => (
						<button
							key={skill}
							type="button"
							disabled={busy}
							aria-label={`Ajouter ${skill}`}
							onClick={() => onAdd(skill)}
							className="rounded-full border bg-secondary px-3 py-1 text-[13px] text-muted-foreground hover:text-foreground"
						>
							+ {skill}
						</button>
					))}
				</div>
			) : null}
		</DetailSection>
	)
}
