import { type FormEvent, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '#/components/ui/dialog'
import { slugify } from '#/domain/agent-team/agent-identity'
import { type SkillDraft, SLUG_MAX, validateDraft } from '#/domain/skills/model'

interface Props {
	open: boolean
	mode: 'create' | 'edit'
	initial: SkillDraft
	pending: boolean
	error: string | null
	onSubmit: (draft: SkillDraft) => void
	onClose: () => void
}

const CONTROL =
	'rounded-[6px] border bg-background px-[9px] py-1.5 text-[13px] text-foreground outline-none focus-visible:border-primary'
const LABEL = 'grid gap-1 text-xs text-muted-foreground'

export function SkillFormDialog({ open, onClose, ...props }: Props) {
	return (
		<Dialog open={open} onOpenChange={(next) => (next ? null : onClose())}>
			<DialogContent aria-describedby={undefined} className="sm:max-w-[640px]">
				{open ? <SkillForm onClose={onClose} {...props} /> : null}
			</DialogContent>
		</Dialog>
	)
}

function SkillForm({
	mode,
	initial,
	pending,
	error,
	onSubmit,
	onClose,
}: Omit<Props, 'open'>) {
	const [draft, setDraft] = useState<SkillDraft>(initial)
	const [slugTouched, setSlugTouched] = useState(mode === 'edit')
	const [invalid, setInvalid] = useState<string | null>(null)
	const title =
		mode === 'create' ? 'Nouvelle compétence' : 'Modifier la compétence'

	const patch = (change: Partial<SkillDraft>) =>
		setDraft((current) => ({ ...current, ...change }))

	const submit = (event: FormEvent) => {
		event.preventDefault()
		const problem = validateDraft(draft)
		setInvalid(problem)
		if (!problem) onSubmit(draft)
	}

	return (
		<form onSubmit={submit} aria-label={title} className="grid gap-3">
			<DialogHeader>
				<DialogTitle>{title}</DialogTitle>
				<DialogDescription>
					Les agents l'utilisent par son slug : la renommer casse les agents qui
					la référencent.
				</DialogDescription>
			</DialogHeader>
			{invalid || error ? (
				<InlineAlert tone="danger">{invalid ?? error}</InlineAlert>
			) : null}
			<div className="grid gap-3 sm:grid-cols-2">
				<label htmlFor="skill-name" className={LABEL}>
					Nom
					<input
						id="skill-name"
						value={draft.name}
						onChange={(event) => {
							const name = event.target.value
							patch(
								slugTouched
									? { name }
									: { name, slug: slugify(name).slice(0, SLUG_MAX) },
							)
						}}
						className={CONTROL}
					/>
				</label>
				<label htmlFor="skill-slug" className={LABEL}>
					Slug
					<input
						id="skill-slug"
						value={draft.slug}
						maxLength={SLUG_MAX}
						placeholder="revue-contrat"
						onChange={(event) => {
							setSlugTouched(true)
							patch({ slug: event.target.value })
						}}
						className={`${CONTROL} font-mono`}
					/>
				</label>
			</div>
			<label htmlFor="skill-description" className={LABEL}>
				Description
				<input
					id="skill-description"
					value={draft.description}
					onChange={(event) => patch({ description: event.target.value })}
					className={CONTROL}
				/>
			</label>
			<label htmlFor="skill-source" className={LABEL}>
				Source (« manual » ou URL http(s))
				<input
					id="skill-source"
					value={draft.source}
					onChange={(event) => patch({ source: event.target.value })}
					className={`${CONTROL} font-mono`}
				/>
			</label>
			<label htmlFor="skill-content" className={LABEL}>
				Contenu (Markdown)
				<textarea
					id="skill-content"
					rows={10}
					value={draft.content}
					onChange={(event) => patch({ content: event.target.value })}
					className={`${CONTROL} font-mono text-[12.5px]`}
				/>
			</label>
			<DialogFooter>
				<Button type="button" variant="outline" onClick={onClose}>
					Annuler
				</Button>
				<Button type="submit" disabled={pending}>
					{pending
						? 'Enregistrement…'
						: mode === 'create'
							? 'Créer'
							: 'Enregistrer'}
				</Button>
			</DialogFooter>
		</form>
	)
}
