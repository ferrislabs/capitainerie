import { Plus, X } from 'lucide-react'
import type { FormEvent } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Checkbox } from '#/components/ui/checkbox'
import { Combobox } from '#/components/ui/combobox'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '#/components/ui/dialog'
import { Field } from '#/components/ui/field'
import { Skeleton } from '#/components/ui/skeleton'
import { Textarea } from '#/components/ui/textarea'

export type RepositoriesState =
	| { kind: 'loading' }
	| { kind: 'not_connected' }
	| { kind: 'error' }
	| { kind: 'ready'; repositories: ReadonlyArray<Schemas.RepositoryResponse> }

export type IssuesState =
	| { kind: 'idle' }
	| { kind: 'loading' }
	| { kind: 'error'; message: string }
	| { kind: 'ready'; issues: ReadonlyArray<Schemas.IssueResponse> }

export const MAX_WORKSITES = 20

interface Props {
	open: boolean
	projectName: string
	repositories: RepositoriesState
	repository: string
	prompts: ReadonlyArray<string>
	issues: IssuesState
	selectedIssues: ReadonlyArray<number>
	errorMessage?: string
	isPending: boolean
	onRepositoryChange: (fullName: string) => void
	onPromptChange: (index: number, value: string) => void
	onAddPrompt: () => void
	onRemovePrompt: (index: number) => void
	onToggleIssue: (number: number, selected: boolean) => void
	onRetry: () => void
	onSubmit: () => void
	onClose: () => void
}

export function countWorksites(
	prompts: ReadonlyArray<string>,
	selectedIssues: ReadonlyArray<number>,
): number {
	return (
		prompts.filter((prompt) => prompt.trim() !== '').length +
		selectedIssues.length
	)
}

function IssuesField({
	state,
	selected,
	disabled,
	onToggle,
}: {
	state: IssuesState
	selected: ReadonlyArray<number>
	disabled: boolean
	onToggle: (number: number, selected: boolean) => void
}) {
	if (state.kind === 'idle') {
		return (
			<p className="text-[13px] text-muted-foreground">
				Choisis un dépôt pour voir ses issues ouvertes.
			</p>
		)
	}
	if (state.kind === 'loading') {
		return (
			<output className="flex flex-col gap-2">
				<span className="sr-only">Chargement des issues…</span>
				<Skeleton className="h-6 w-full" />
				<Skeleton className="h-6 w-full" />
			</output>
		)
	}
	if (state.kind === 'error') {
		return <InlineAlert tone="danger">{state.message}</InlineAlert>
	}
	if (state.issues.length === 0) {
		return (
			<p className="text-[13px] text-muted-foreground">
				Ce dépôt n'a aucune issue ouverte.
			</p>
		)
	}
	return (
		<ul className="flex max-h-56 flex-col gap-1 overflow-y-auto rounded-md border p-2">
			{state.issues.map((issue) => {
				const taken = issue.card_id != null
				return (
					<li key={issue.number}>
						<Checkbox
							label={
								<span className="flex flex-wrap items-baseline gap-x-2">
									<span className="figures text-muted-foreground">
										#{issue.number}
									</span>
									<span>{issue.title}</span>
									{taken ? (
										<span className="text-xs text-muted-foreground">
											déjà un chantier
										</span>
									) : null}
								</span>
							}
							checked={selected.includes(issue.number)}
							disabled={disabled || taken}
							onCheckedChange={(checked) =>
								onToggle(issue.number, checked === true)
							}
						/>
					</li>
				)
			})}
		</ul>
	)
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
				L'application GitHub n'a accès à aucun dépôt. Ajoutes-en depuis les
				réglages de l'installation sur GitHub.
			</InlineAlert>
		)
	}
	if (state.kind !== 'ready') return null
	return (
		<Field label="Dépôt" id="worksite-repository">
			<Combobox
				className="font-mono text-xs"
				disabled={disabled}
				items={state.repositories.map((repository) => ({
					value: repository.full_name,
					label: repository.full_name,
				}))}
				value={value}
				onValueChange={onChange}
				placeholder="Choisir un dépôt…"
				emptyText="Aucun dépôt ne correspond"
			/>
		</Field>
	)
}

export function LaunchWorksitesDialog({
	open,
	projectName,
	repositories,
	repository,
	prompts,
	issues,
	selectedIssues,
	errorMessage,
	isPending,
	onRepositoryChange,
	onPromptChange,
	onAddPrompt,
	onRemovePrompt,
	onToggleIssue,
	onRetry,
	onSubmit,
	onClose,
}: Props) {
	const count = countWorksites(prompts, selectedIssues)
	const tooMany = count > MAX_WORKSITES
	const canSubmit =
		repositories.kind === 'ready' &&
		repository !== '' &&
		count > 0 &&
		!tooMany &&
		!isPending

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
			<DialogContent className="max-w-xl">
				<form
					onSubmit={handleSubmit}
					className="flex flex-col gap-4"
					noValidate
				>
					<DialogHeader>
						<DialogTitle>Nouveau chantier</DialogTitle>
						<DialogDescription>
							Chaque prompt et chaque issue devient une tâche du projet «{' '}
							{projectName} » confiée à Claude Code, sur le dépôt choisi.
						</DialogDescription>
					</DialogHeader>
					{repositories.kind === 'not_connected' ? (
						<InlineAlert tone="info">
							GitHub n'est pas connecté à l'organisation : un chantier a besoin
							d'un dépôt accessible par l'application GitHub.
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
							<fieldset className="flex flex-col gap-2">
								<legend className="mb-1 text-sm font-medium">Prompts</legend>
								{prompts.map((prompt, index) => (
									<div
										// biome-ignore lint/suspicious/noArrayIndexKey: la liste n'a pas d'identité propre
										key={index}
										className="flex items-start gap-2"
									>
										<Textarea
											aria-label={`Prompt ${index + 1}`}
											placeholder="Décris le travail à faire. La première ligne devient le titre de la tâche."
											value={prompt}
											disabled={isPending}
											onChange={(event) =>
												onPromptChange(index, event.target.value)
											}
											autoGrow
											maxRows={8}
											className="flex-1"
										/>
										{prompts.length > 1 ? (
											<Button
												type="button"
												size="icon"
												variant="ghost"
												aria-label={`Retirer le prompt ${index + 1}`}
												disabled={isPending}
												onClick={() => onRemovePrompt(index)}
											>
												<X aria-hidden="true" />
											</Button>
										) : null}
									</div>
								))}
								<Button
									type="button"
									size="sm"
									variant="outline"
									className="self-start"
									disabled={isPending}
									onClick={onAddPrompt}
								>
									<Plus aria-hidden="true" />
									Ajouter un prompt
								</Button>
							</fieldset>
							<fieldset className="flex flex-col gap-2">
								<legend className="mb-1 text-sm font-medium">
									Issues GitHub
								</legend>
								<IssuesField
									state={issues}
									selected={selectedIssues}
									disabled={isPending}
									onToggle={onToggleIssue}
								/>
							</fieldset>
							<InlineAlert tone={tooMany ? 'danger' : 'info'}>
								{tooMany
									? `Au plus ${MAX_WORKSITES} chantiers à la fois : il y en a ${count}.`
									: `Claude Code travaille dans un conteneur isolé, une branche capitainerie/… par chantier, et ouvre une pull request en brouillon. ${count} chantier${count > 1 ? 's' : ''} ${count > 1 ? 'partiront' : 'partira'} ; au-delà de la capacité des workers, ils attendent leur tour.`}
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
								{isPending
									? 'Envoi…'
									: count > 1
										? `Lancer ${count} chantiers`
										: 'Lancer le chantier'}
							</Button>
						)}
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	)
}
