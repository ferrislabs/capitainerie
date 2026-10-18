import type { FormEvent } from 'react'
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
import {
	BASE_URL_PLACEHOLDER,
	PROVIDER_INFO,
	type ProviderKind,
} from '#/domain/settings/providers'

interface Props {
	open: boolean
	kind: ProviderKind | undefined
	isReplacing: boolean
	apiKey: string
	baseUrl: string
	apiKeyError?: string
	baseUrlError?: string
	serverError?: string
	isPending: boolean
	onApiKeyChange: (value: string) => void
	onBaseUrlChange: (value: string) => void
	onSubmit: () => void
	onClose: () => void
}

export function ProviderKeyDialog({
	open,
	kind,
	isReplacing,
	apiKey,
	baseUrl,
	apiKeyError,
	baseUrlError,
	serverError,
	isPending,
	onApiKeyChange,
	onBaseUrlChange,
	onSubmit,
	onClose,
}: Props) {
	const info = kind ? PROVIDER_INFO[kind] : undefined

	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		onSubmit()
	}

	return (
		<Dialog
			open={open && kind !== undefined}
			onOpenChange={(next) => {
				if (!next && !isPending) onClose()
			}}
		>
			<DialogContent>
				{info ? (
					<form
						onSubmit={handleSubmit}
						className="flex flex-col gap-4"
						noValidate
					>
						<DialogHeader>
							<DialogTitle>
								{isReplacing ? 'Modifier' : 'Configurer'} {info.label}
							</DialogTitle>
							<DialogDescription>
								{info.help} La clé est chiffrée et ne sera plus jamais affichée
								: seuls ses 4 derniers caractères restent visibles.
							</DialogDescription>
						</DialogHeader>
						{info.usesBaseUrl ? (
							<div className="flex flex-col gap-1.5">
								<Label htmlFor="provider-base-url">URL de base</Label>
								<Input
									id="provider-base-url"
									type="url"
									inputMode="url"
									autoComplete="off"
									spellCheck={false}
									className="font-mono"
									placeholder={BASE_URL_PLACEHOLDER}
									value={baseUrl}
									onChange={(event) => onBaseUrlChange(event.target.value)}
									disabled={isPending}
									aria-invalid={Boolean(baseUrlError)}
									aria-describedby={
										baseUrlError ? 'provider-base-url-error' : undefined
									}
								/>
								{baseUrlError ? (
									<span
										id="provider-base-url-error"
										role="alert"
										className="text-xs text-signal-danger"
									>
										{baseUrlError}
									</span>
								) : null}
							</div>
						) : null}
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="provider-api-key">
								Clé d’API
								{info.keyRequired ? null : (
									<span className="font-normal text-muted-foreground">
										(optionnelle)
									</span>
								)}
							</Label>
							<Input
								id="provider-api-key"
								type="password"
								autoComplete="off"
								spellCheck={false}
								data-1p-ignore
								data-lpignore="true"
								className="font-mono"
								value={apiKey}
								onChange={(event) => onApiKeyChange(event.target.value)}
								disabled={isPending}
								aria-invalid={Boolean(apiKeyError)}
								aria-describedby="provider-api-key-hint"
							/>
							<span
								id="provider-api-key-hint"
								className="text-xs text-muted-foreground"
							>
								{info.keyRequired
									? isReplacing
										? 'La nouvelle clé remplace l’ancienne.'
										: 'Collez la clé telle que le fournisseur l’a créée.'
									: 'Laissez vide si l’endpoint n’en demande pas. Enregistrer remplace la clé actuelle.'}
							</span>
							{apiKeyError ? (
								<span role="alert" className="text-xs text-signal-danger">
									{apiKeyError}
								</span>
							) : null}
						</div>
						{serverError ? (
							<InlineAlert tone="danger">{serverError}</InlineAlert>
						) : null}
						<DialogFooter>
							<Button
								type="button"
								variant="outline"
								disabled={isPending}
								onClick={onClose}
							>
								Annuler
							</Button>
							<Button type="submit" disabled={isPending}>
								{isPending ? 'Enregistrement…' : 'Enregistrer'}
							</Button>
						</DialogFooter>
					</form>
				) : null}
			</DialogContent>
		</Dialog>
	)
}
