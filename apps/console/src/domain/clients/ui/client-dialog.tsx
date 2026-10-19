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

export interface ClientValues {
	name: string
	aliases: string[]
	notes: string
}

interface Props {
	open: boolean
	client?: Schemas.ClientResponse
	isPending: boolean
	errorMessage?: string
	onSubmit: (values: ClientValues) => void
	onClose: () => void
}

function ClientForm({
	client,
	isPending,
	errorMessage,
	onSubmit,
	onClose,
}: Omit<Props, 'open'>) {
	const [name, setName] = useState(client?.name ?? '')
	const [aliases, setAliases] = useState(formatAliases(client?.aliases ?? []))
	const [notes, setNotes] = useState(client?.notes ?? '')
	const [attempted, setAttempted] = useState(false)
	const parsed = parseAliases(aliases)
	const nameError = validateName(name)
	const aliasError = validateAliases(parsed)
	const notesError = validateText(notes)
	const invalid = Boolean(nameError || aliasError || notesError)

	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		setAttempted(true)
		if (invalid) return
		onSubmit({ name: name.trim(), aliases: parsed, notes })
	}

	return (
		<form onSubmit={handleSubmit} className="flex flex-col gap-4" noValidate>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="client-name">Nom</Label>
				<Input
					id="client-name"
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
				<Label htmlFor="client-aliases">Alias</Label>
				<Input
					id="client-aliases"
					value={aliases}
					onChange={(event) => setAliases(event.target.value)}
					disabled={isPending}
					placeholder="Séparés par des virgules"
					aria-invalid={Boolean(aliasError)}
				/>
				<span
					role={aliasError ? 'alert' : undefined}
					className={
						aliasError
							? 'text-xs text-signal-danger'
							: 'text-xs text-muted-foreground'
					}
				>
					{aliasError ??
						'Les autres noms sous lesquels la réunion en parle : ils aident la détection.'}
				</span>
			</div>
			<div className="flex flex-col gap-1.5">
				<Label htmlFor="client-notes">Notes</Label>
				<Textarea
					id="client-notes"
					className="max-h-48 min-h-20"
					value={notes}
					onChange={(event) => setNotes(event.target.value)}
					disabled={isPending}
					aria-invalid={Boolean(notesError)}
				/>
				<span className="ml-auto text-xs text-muted-foreground figures">
					{[...notes].length} / {TEXT_MAX_LENGTH}
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
					{isPending ? 'Enregistrement…' : client ? 'Enregistrer' : 'Créer'}
				</Button>
			</DialogFooter>
		</form>
	)
}

export function ClientDialog({ open, ...form }: Props) {
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
						{form.client ? 'Modifier le client' : 'Nouveau client'}
					</DialogTitle>
					<DialogDescription>
						Une organisation ou une personne pour qui l’équipe travaille.
					</DialogDescription>
				</DialogHeader>
				<ClientForm key={form.client?.id ?? 'new'} {...form} />
			</DialogContent>
		</Dialog>
	)
}
