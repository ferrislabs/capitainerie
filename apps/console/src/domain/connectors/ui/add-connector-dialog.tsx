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

interface Props {
	open: boolean
	pending: boolean
	error: string | null
	onSubmit: (draft: { url: string; name: string }) => void
	onClose: () => void
}

const CONTROL =
	'rounded-[6px] border bg-background px-[9px] py-1.5 text-[13px] text-foreground outline-none focus-visible:border-primary'
const LABEL = 'grid gap-1 text-xs text-muted-foreground'

export function AddConnectorDialog({
	open,
	pending,
	error,
	onSubmit,
	onClose,
}: Props) {
	return (
		<Dialog open={open} onOpenChange={(next) => (next ? null : onClose())}>
			<DialogContent aria-describedby={undefined}>
				{open ? (
					<AddForm
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

function AddForm({ pending, error, onSubmit, onClose }: Omit<Props, 'open'>) {
	const [url, setUrl] = useState('')
	const [name, setName] = useState('')
	const [invalid, setInvalid] = useState<string | null>(null)

	const submit = (event: FormEvent) => {
		event.preventDefault()
		const trimmed = url.trim()
		const valid = /^https?:\/\/[^\s/]+/i.test(trimmed)
		setInvalid(valid ? null : "L'URL MCP doit commencer par https://")
		if (valid) onSubmit({ url: trimmed, name: name.trim() })
	}

	return (
		<form
			onSubmit={submit}
			aria-label="Ajouter un connecteur"
			className="grid gap-3"
		>
			<DialogHeader>
				<DialogTitle>Ajouter un connecteur</DialogTitle>
				<DialogDescription>
					Indique l'URL d'un serveur MCP : la découverte OAuth2 et
					l'enregistrement du client sont automatiques.
				</DialogDescription>
			</DialogHeader>
			{invalid || error ? (
				<InlineAlert tone="danger">{invalid ?? error}</InlineAlert>
			) : null}
			<label htmlFor="connector-url" className={LABEL}>
				URL du serveur MCP
				<input
					id="connector-url"
					value={url}
					placeholder="https://mcp.example.com/mcp"
					onChange={(event) => setUrl(event.target.value)}
					className={CONTROL}
				/>
			</label>
			<label htmlFor="connector-name" className={LABEL}>
				Nom (optionnel)
				<input
					id="connector-name"
					value={name}
					onChange={(event) => setName(event.target.value)}
					className={CONTROL}
				/>
			</label>
			<DialogFooter>
				<Button type="button" variant="outline" onClick={onClose}>
					Annuler
				</Button>
				<Button type="submit" disabled={pending}>
					{pending ? 'Ajout…' : 'Ajouter'}
				</Button>
			</DialogFooter>
		</form>
	)
}
