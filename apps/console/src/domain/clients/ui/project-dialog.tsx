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
import { Input } from '#/components/ui/input'
import { Label } from '#/components/ui/label'
import { Textarea } from '#/components/ui/textarea'
import {
	formatAliases,
	parseAliases,
	TEXT_MAX_LENGTH,
	validateAliases,
	validateName,
	validateText,
} from '#/domain/clients/aliases'

export interface ProjectValues {
	name: string
	clientId: string
	aliases: string[]
	repository: string
	description: string
}

interface Props {
	open: boolean
	project?: Schemas.ProjectResponse
	defaultClientId?: string
	clients: ReadonlyArray<Schemas.ClientResponse>
	isPending: boolean
	errorMessage?: string
	onSubmit: (values: ProjectValues) => void
	onClose: () => void
}

function ProjectForm({
	project,
	defaultClientId,
	clients,
	isPending,
	errorMessage,
	onSubmit,
	onClose,
}: Omit<Props, 'open'>) {
	const [name, setName] = useState(project?.name ?? '')
	const [clientId, setClientId] = useState(
		project?.client?.id ?? defaultClientId ?? '',
	)
	const [aliases, setAliases] = useState(formatAliases(project?.aliases ?? []))
	const [repository, setRepository] = useState(project?.repository ?? '')
	const [description, setDescription] = useState(project?.description ?? '')
	const [attempted, setAttempted] = useState(false)
	const parsed = parseAliases(aliases)
	const nameError = validateName(name)
	const aliasError = validateAliases(parsed)
	const descriptionError = validateText(description)
	const invalid = Boolean(nameError || aliasError || descriptionError)

	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		setAttempted(true)
		if (invalid) return
		onSubmit({
			name: name.trim(),
			clientId,
			aliases: parsed,
			repository: repository.trim(),
			description,
		})
	}

	return (
		<form onSubmit={handleSubmit} className="flex flex-col gap-4" noValidate>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="project-name">Nom</Label>
				<Input
					id="project-name"
					value={name}
					onChange={(event) => setName(event.target.value)}
					disabled={isPending}
					aria-invalid={attempted && Boolean(nameError)}
				/>
				{attempted && nameError ? (
					<span role="alert" className="text-xs text-signal-danger">
						{nameError}
					</span>
				) : null}
			</div>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="project-client">Client</Label>
				<select
					id="project-client"
					className="h-8 w-full rounded-md border border-input bg-card px-3 text-sm text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 focus-visible:outline-none disabled:opacity-60"
					value={clientId}
					onChange={(event) => setClientId(event.target.value)}
					disabled={isPending}
				>
					<option value="">Aucun (projet interne)</option>
					{clients.map((client) => (
						<option key={client.id} value={client.id}>
							{client.name}
						</option>
					))}
				</select>
			</div>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="project-aliases">Alias</Label>
				<Input
					id="project-aliases"
					value={aliases}
					onChange={(event) => setAliases(event.target.value)}
					disabled={isPending}
					placeholder="Séparés par des virgules"
					aria-invalid={Boolean(aliasError)}
				/>
				{aliasError ? (
					<span role="alert" className="text-xs text-signal-danger">
						{aliasError}
					</span>
				) : null}
			</div>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="project-repository">Dépôt par défaut</Label>
				<Input
					id="project-repository"
					className="font-mono text-xs"
					value={repository}
					onChange={(event) => setRepository(event.target.value)}
					disabled={isPending}
					placeholder="propriétaire/nom"
				/>
			</div>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="project-description">Description</Label>
				<Textarea
					id="project-description"
					className="max-h-48 min-h-20"
					value={description}
					onChange={(event) => setDescription(event.target.value)}
					disabled={isPending}
					aria-invalid={Boolean(descriptionError)}
				/>
				<span className="ml-auto text-xs text-muted-foreground figures">
					{[...description].length} / {TEXT_MAX_LENGTH}
				</span>
			</div>
			{errorMessage ? (
				<InlineAlert tone="danger">{errorMessage}</InlineAlert>
			) : null}
			<DialogFooter>
				<Button
					type="button"
					variant="ghost"
					disabled={isPending}
					onClick={onClose}
				>
					Annuler
				</Button>
				<Button type="submit" disabled={isPending}>
					{isPending ? 'Enregistrement…' : project ? 'Enregistrer' : 'Créer'}
				</Button>
			</DialogFooter>
		</form>
	)
}

export function ProjectDialog({ open, ...form }: Props) {
	return (
		<Dialog
			open={open}
			onOpenChange={(next) => {
				if (!next && !form.isPending) form.onClose()
			}}
		>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>
						{form.project ? 'Modifier le projet' : 'Nouveau projet'}
					</DialogTitle>
					<DialogDescription>
						Un travail suivi pour un client, avec un dépôt GitHub par défaut
						facultatif.
					</DialogDescription>
				</DialogHeader>
				<ProjectForm key={form.project?.id ?? 'new'} {...form} />
			</DialogContent>
		</Dialog>
	)
}
