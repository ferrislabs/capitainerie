import { Link } from '@tanstack/react-router'
import { ChevronDown } from 'lucide-react'
import type { FormEvent } from 'react'
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
import { Label } from '#/components/ui/label'
import { Skeleton } from '#/components/ui/skeleton'

export type RepositoriesState =
	| { kind: 'loading' }
	| { kind: 'error' }
	| { kind: 'not_connected' }
	| { kind: 'ready'; repositories: ReadonlyArray<Schemas.RepositoryResponse> }

interface Props {
	open: boolean
	cardTitle: string
	repositories: RepositoriesState
	repository: string
	errorMessage?: string
	isPending: boolean
	onRepositoryChange: (fullName: string) => void
	onRetry: () => void
	onSubmit: () => void
	onClose: () => void
}

function RepositoryField({
	state,
	value,
	disabled,
	onChange,
	onRetry,
}: {
	state: RepositoriesState
	value: string
	disabled: boolean
	onChange: (fullName: string) => void
	onRetry: () => void
}) {
	if (state.kind === 'loading') {
		return (
			<output className="flex flex-col gap-1.5">
				<span className="sr-only">Chargement des dépôts…</span>
				<Skeleton className="h-3 w-12" />
				<Skeleton className="h-8 w-full" />
			</output>
		)
	}
	if (state.kind === 'error') {
		return (
			<div className="flex flex-col items-start gap-2">
				<InlineAlert tone="danger" className="w-full">
					Impossible de charger les dépôts GitHub.
				</InlineAlert>
				<Button type="button" size="sm" variant="outline" onClick={onRetry}>
					Réessayer
				</Button>
			</div>
		)
	}
	if (state.kind === 'ready' && state.repositories.length === 0) {
		return (
			<InlineAlert tone="info">
				L’application GitHub n’a accès à aucun dépôt. Ajoutez-en depuis les
				réglages de l’installation sur GitHub.
			</InlineAlert>
		)
	}
	if (state.kind !== 'ready') return null

	return (
		<div className="flex flex-col gap-1.5">
			<Label htmlFor="execute-repository">Dépôt</Label>
			<div className="relative">
				<select
					id="execute-repository"
					className="h-8 w-full appearance-none truncate rounded-md border border-input bg-card pr-8 pl-3 font-mono text-xs text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-60"
					disabled={disabled}
					value={value}
					onChange={(event) => onChange(event.target.value)}
				>
					<option value="" disabled>
						Choisir un dépôt…
					</option>
					{state.repositories.map((repository) => (
						<option key={repository.full_name} value={repository.full_name}>
							{repository.full_name}
						</option>
					))}
				</select>
				<ChevronDown
					aria-hidden="true"
					className="pointer-events-none absolute top-1/2 right-2.5 size-3.5 -translate-y-1/2 text-muted-foreground"
				/>
			</div>
		</div>
	)
}

export function ExecuteCardDialog({
	open,
	cardTitle,
	repositories,
	repository,
	errorMessage,
	isPending,
	onRepositoryChange,
	onRetry,
	onSubmit,
	onClose,
}: Props) {
	const canSubmit =
		repositories.kind === 'ready' && repository !== '' && !isPending

	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		if (canSubmit) onSubmit()
	}

	return (
		<Dialog
			open={open}
			onOpenChange={(next) => {
				if (!next && !isPending) onClose()
			}}
		>
			<DialogContent>
				<form
					onSubmit={handleSubmit}
					className="flex flex-col gap-4"
					noValidate
				>
					<DialogHeader>
						<DialogTitle>Confier à Claude Code</DialogTitle>
						<DialogDescription>
							Claude Code prend en charge la carte « {cardTitle} » sur le dépôt
							choisi.
						</DialogDescription>
					</DialogHeader>
					{repositories.kind === 'not_connected' ? (
						<InlineAlert tone="info">
							<span>
								GitHub n’est pas connecté à l’organisation.{' '}
								<Link
									to="/settings/integrations"
									className="underline underline-offset-2"
								>
									Paramètres › Intégrations
								</Link>
							</span>
						</InlineAlert>
					) : (
						<>
							<RepositoryField
								state={repositories}
								value={repository}
								disabled={isPending}
								onChange={onRepositoryChange}
								onRetry={onRetry}
							/>
							<InlineAlert tone="info">
								Claude Code travaille dans un conteneur isolé, pousse une
								branche{' '}
								<span className="font-mono text-xs">capitainerie/…</span> et
								ouvre une pull request en brouillon ; les workflows GitHub
								Actions déclenchés par cette branche s’exécutent avec les
								secrets du dépôt — limitez-les aux branches protégées.
							</InlineAlert>
						</>
					)}
					{errorMessage ? (
						<InlineAlert tone="danger">{errorMessage}</InlineAlert>
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
						{repositories.kind === 'not_connected' ? null : (
							<Button type="submit" disabled={!canSubmit}>
								{isPending ? 'Envoi…' : 'Confier'}
							</Button>
						)}
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	)
}
