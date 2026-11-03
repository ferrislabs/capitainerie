import { type FormEvent, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Select } from '#/components/ui/combobox'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '#/components/ui/dialog'
import { Field } from '#/components/ui/field'
import { Input } from '#/components/ui/input'
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
			<Field
				label="Nom"
				id="project-name"
				error={attempted ? nameError : undefined}
			>
				<Input
					value={name}
					onChange={(event) => setName(event.target.value)}
					disabled={isPending}
				/>
			</Field>
			<Field label="Client" id="project-client">
				<Select
					items={clients.map((client) => ({
						value: client.id,
						label: client.name,
					}))}
					value={clientId}
					onValueChange={setClientId}
					noneLabel="Aucun (projet interne)"
					disabled={isPending}
				/>
			</Field>
			<Field label="Alias" id="project-aliases" error={aliasError}>
				<Input
					value={aliases}
					onChange={(event) => setAliases(event.target.value)}
					disabled={isPending}
					placeholder="Séparés par des virgules"
				/>
			</Field>
			<Field label="Dépôt par défaut" id="project-repository">
				<Input
					className="font-mono text-xs"
					value={repository}
					onChange={(event) => setRepository(event.target.value)}
					disabled={isPending}
					placeholder="propriétaire/nom"
				/>
			</Field>
			<Field
				label="Description"
				id="project-description"
				count={[...description].length}
				max={TEXT_MAX_LENGTH}
				error={descriptionError}
			>
				<Textarea
					maxRows={8}
					rows={3}
					className="min-h-20"
					value={description}
					onChange={(event) => setDescription(event.target.value)}
					disabled={isPending}
				/>
			</Field>
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
