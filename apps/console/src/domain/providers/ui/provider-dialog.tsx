import { type FormEvent, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Checkbox } from '#/components/ui/checkbox'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '#/components/ui/dialog'
import {
	type ProviderDraft,
	type ProviderSpec,
	validateDraft,
} from '#/domain/providers/model'

interface Props {
	spec: ProviderSpec | null
	existing: Schemas.UserProviderResponse | undefined
	firstProvider: boolean
	pending: boolean
	error: string | null
	onSubmit: (spec: ProviderSpec, draft: ProviderDraft) => void
	onClose: () => void
}

const CONTROL =
	'rounded-[6px] border bg-background px-[9px] py-1.5 text-[13px] text-foreground outline-none focus-visible:border-primary'
const LABEL = 'grid gap-1 text-xs text-muted-foreground'

export function ProviderDialog({ spec, onClose, ...rest }: Props) {
	return (
		<Dialog
			open={spec !== null}
			onOpenChange={(next) => (next ? null : onClose())}
		>
			<DialogContent aria-describedby={undefined}>
				{spec ? <ProviderForm spec={spec} onClose={onClose} {...rest} /> : null}
			</DialogContent>
		</Dialog>
	)
}

function ProviderForm({
	spec,
	existing,
	firstProvider,
	pending,
	error,
	onSubmit,
	onClose,
}: Omit<Props, 'spec'> & { spec: ProviderSpec }) {
	const [apiKey, setApiKey] = useState('')
	const [baseUrl, setBaseUrl] = useState(existing?.base_url ?? '')
	const [model, setModel] = useState(existing?.default_model ?? '')
	const [isDefault, setIsDefault] = useState(
		existing?.is_default ?? firstProvider,
	)
	const [invalid, setInvalid] = useState<string | null>(null)

	const submit = (event: FormEvent) => {
		event.preventDefault()
		const draft = { apiKey, baseUrl, model, isDefault }
		const problem = validateDraft(spec, draft)
		setInvalid(problem)
		if (problem) return
		onSubmit(spec, draft)
		setApiKey('')
	}

	const title = `${existing ? 'Modifier' : 'Connecter'} ${spec.name}`
	return (
		<form onSubmit={submit} aria-label={title} className="grid gap-3">
			<DialogHeader>
				<DialogTitle>{title}</DialogTitle>
				<DialogDescription>
					Votre clé est chiffrée côté serveur et ne sera plus jamais affichée.
					{existing
						? ' Saisissez-la à nouveau pour modifier ce fournisseur.'
						: ''}
				</DialogDescription>
			</DialogHeader>
			{invalid || error ? (
				<InlineAlert tone="danger">{invalid ?? error}</InlineAlert>
			) : null}
			<label htmlFor="provider-key" className={LABEL}>
				Clé API
				<input
					id="provider-key"
					type="password"
					autoComplete="off"
					spellCheck={false}
					value={apiKey}
					onChange={(event) => setApiKey(event.target.value)}
					className={CONTROL}
				/>
			</label>
			{spec.needsBaseUrl ? (
				<label htmlFor="provider-base-url" className={LABEL}>
					URL de base
					<input
						id="provider-base-url"
						value={baseUrl}
						placeholder="https://…/v1"
						onChange={(event) => setBaseUrl(event.target.value)}
						className={CONTROL}
					/>
					{spec.baseUrlHelp ? <span>{spec.baseUrlHelp}</span> : null}
				</label>
			) : null}
			<label htmlFor="provider-model" className={LABEL}>
				Modèle par défaut
				<input
					id="provider-model"
					value={model}
					placeholder={spec.modelPlaceholder}
					onChange={(event) => setModel(event.target.value)}
					className={CONTROL}
				/>
			</label>
			<Checkbox
				label="Utiliser par défaut"
				checked={isDefault}
				onCheckedChange={(value) => setIsDefault(value === true)}
			/>
			<DialogFooter>
				<Button type="button" variant="outline" onClick={onClose}>
					Annuler
				</Button>
				<Button type="submit" disabled={pending}>
					{pending ? 'Enregistrement…' : 'Enregistrer'}
				</Button>
			</DialogFooter>
		</form>
	)
}
