import { type FormEvent, useState } from 'react'
import type { Schemas } from '#/api/api.client'
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
import {
	EMPTY_DRAFT,
	type ProjectDraft,
	validateDraft,
} from '#/domain/projects/model'

interface Props {
	open: boolean
	mode: 'create' | 'edit'
	initial: ProjectDraft | null
	clients: ReadonlyArray<Schemas.ClientResponse>
	pending: boolean
	error: string | null
	onSubmit: (draft: ProjectDraft) => void
	onClose: () => void
}

const CONTROL =
	'rounded-[6px] border bg-background px-[9px] py-1.5 text-[13px] text-foreground outline-none focus-visible:border-primary'
const LABEL = 'grid gap-1 text-xs text-muted-foreground'

export function ProjectDialog({
	open,
	mode,
	initial,
	clients,
	pending,
	error,
	onSubmit,
	onClose,
}: Props) {
	return (
		<Dialog open={open} onOpenChange={(next) => (next ? null : onClose())}>
			<DialogContent aria-describedby={undefined}>
				{open ? (
					<ProjectForm
						key={initial?.name ?? 'new'}
						mode={mode}
						initial={initial ?? EMPTY_DRAFT}
						clients={clients}
						pending={pending}
						error={error}
						onSubmit={onSubmit}
						onClose={onClose}
					/>
				) : null}
			</DialogContent>
		</Dialog>
	)
}

function ProjectForm({
	mode,
	initial,
	clients,
	pending,
	error,
	onSubmit,
	onClose,
}: Omit<Props, 'open' | 'initial'> & { initial: ProjectDraft }) {
	const [draft, setDraft] = useState(initial)
	const [invalid, setInvalid] = useState<string | null>(null)
	const set = (patch: Partial<ProjectDraft>) =>
		setDraft((current) => ({ ...current, ...patch }))
	const title = mode === 'create' ? 'Nouveau projet' : 'Modifier le projet'

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
					Un projet regroupe les tâches d'un client et son dépôt GitHub.
				</DialogDescription>
			</DialogHeader>
			{invalid || error ? (
				<InlineAlert tone="danger">{invalid ?? error}</InlineAlert>
			) : null}
			<label htmlFor="project-name" className={LABEL}>
				Nom
				<input
					id="project-name"
					value={draft.name}
					onChange={(event) => set({ name: event.target.value })}
					className={CONTROL}
				/>
			</label>
			<label htmlFor="project-description" className={LABEL}>
				Description
				<textarea
					id="project-description"
					rows={3}
					value={draft.description}
					onChange={(event) => set({ description: event.target.value })}
					className={CONTROL}
				/>
			</label>
			<div className="grid gap-2.5 sm:grid-cols-2">
				<label htmlFor="project-owner" className={LABEL}>
					Propriétaire du dépôt
					<input
						id="project-owner"
						value={draft.owner}
						placeholder="acme"
						onChange={(event) => set({ owner: event.target.value })}
						className={CONTROL}
					/>
				</label>
				<label htmlFor="project-repo" className={LABEL}>
					Nom du dépôt
					<input
						id="project-repo"
						value={draft.repo}
						placeholder="site"
						onChange={(event) => set({ repo: event.target.value })}
						className={CONTROL}
					/>
				</label>
			</div>
			<label htmlFor="project-client" className={LABEL}>
				Client (facultatif)
				<select
					id="project-client"
					value={draft.clientId}
					onChange={(event) => set({ clientId: event.target.value })}
					className={CONTROL}
				>
					<option value="">Aucun</option>
					{clients.map((client) => (
						<option key={client.id} value={client.id}>
							{client.name}
						</option>
					))}
				</select>
			</label>
			<DialogFooter>
				<Button type="button" variant="outline" onClick={onClose}>
					Annuler
				</Button>
				<Button type="submit" disabled={pending}>
					{mode === 'create' ? 'Créer le projet' : 'Enregistrer'}
				</Button>
			</DialogFooter>
		</form>
	)
}
