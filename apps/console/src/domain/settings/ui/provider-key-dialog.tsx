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
import { Field } from '#/components/ui/field'
import { Input } from '#/components/ui/input'
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
							<Field
								label="URL de base"
								id="provider-base-url"
								error={baseUrlError}
							>
								<Input
									type="url"
									inputMode="url"
									autoComplete="off"
									spellCheck={false}
									className="font-mono"
									placeholder={BASE_URL_PLACEHOLDER}
									value={baseUrl}
									onChange={(event) => onBaseUrlChange(event.target.value)}
									disabled={isPending}
								/>
							</Field>
						) : null}
						<Field
							label={
								<>
									Clé d’API
									{info.keyRequired ? null : (
										<span className="font-normal text-muted-foreground">
											(optionnelle)
										</span>
									)}
								</>
							}
							id="provider-api-key"
							hint={
								info.keyRequired
									? isReplacing
										? 'La nouvelle clé remplace l’ancienne.'
										: 'Collez la clé telle que le fournisseur l’a créée.'
									: 'Laissez vide si l’endpoint n’en demande pas. Enregistrer remplace la clé actuelle.'
							}
							error={apiKeyError}
						>
							<Input
								type="password"
								autoComplete="off"
								spellCheck={false}
								data-1p-ignore
								data-lpignore="true"
								className="font-mono"
								value={apiKey}
								onChange={(event) => onApiKeyChange(event.target.value)}
								disabled={isPending}
							/>
						</Field>
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
