import { type FormEvent, useId, useState } from 'react'
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
import { Textarea } from '#/components/ui/textarea'
import { KIND_LABELS } from '#/domain/clients/integration-labels'
import { EnvironmentBadge } from '#/domain/integrations/environment-badge'
import {
	canSubmitIntervention,
	type GrantChoice,
	type GrantChoices,
	INSTRUCTIONS_MAX_CHARS,
	type Instance,
	type InterveneForm,
	instructionsLength,
	needsProductionConfirmation,
} from '#/domain/integrations/intervene/grants'
import { cn } from '#/lib/utils'

export type InstancesState =
	| { kind: 'loading' }
	| { kind: 'error' }
	| { kind: 'ready'; instances: ReadonlyArray<Instance> }

interface Props {
	open: boolean
	cardTitle: string
	instances: InstancesState
	errorMessage?: string
	isPending: boolean
	onRetry: () => void
	onSubmit: (form: InterveneForm) => void
	onClose: () => void
}

const CHOICES: ReadonlyArray<{ value: GrantChoice; label: string }> = [
	{ value: 'none', label: 'Aucun' },
	{ value: 'read', label: 'Lecture' },
	{ value: 'write', label: 'Écriture' },
]

function InstanceRow({
	instance,
	value,
	disabled,
	onChange,
}: {
	instance: Instance
	value: GrantChoice
	disabled: boolean
	onChange: (choice: GrantChoice) => void
}) {
	const nameId = useId()
	return (
		<li className="flex flex-col gap-2 px-3 py-2.5 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
			<div className="flex min-w-0 flex-col gap-1">
				<span id={nameId} className="truncate text-[13px] font-medium">
					{instance.name}
				</span>
				<span className="flex flex-wrap items-center gap-1.5">
					<span className="text-xs text-muted-foreground">
						{KIND_LABELS[instance.kind]}
					</span>
					<EnvironmentBadge environment={instance.environment} />
				</span>
			</div>
			<div
				role="radiogroup"
				aria-labelledby={nameId}
				className="inline-flex w-fit shrink-0 divide-x overflow-hidden rounded-md border border-input"
			>
				{CHOICES.map((choice) => (
					<label
						key={choice.value}
						className={cn(
							'flex h-7 cursor-pointer items-center px-2.5 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground has-[:checked]:bg-primary has-[:checked]:text-primary-foreground has-[:focus-visible]:ring-[3px] has-[:focus-visible]:ring-ring/25',
							disabled && 'cursor-not-allowed opacity-60',
						)}
					>
						<input
							type="radio"
							className="sr-only"
							name={`grant-${instance.id}`}
							value={choice.value}
							checked={value === choice.value}
							disabled={disabled}
							onChange={() => onChange(choice.value)}
						/>
						{choice.label}
					</label>
				))}
			</div>
		</li>
	)
}

function InstancesField({
	state,
	choices,
	disabled,
	onChange,
	onRetry,
}: {
	state: InstancesState
	choices: GrantChoices
	disabled: boolean
	onChange: (instance: Instance, choice: GrantChoice) => void
	onRetry: () => void
}) {
	if (state.kind === 'loading') {
		return (
			<output className="flex flex-col gap-1.5">
				<span className="sr-only">Chargement des intégrations…</span>
				<Skeleton className="h-12 w-full" />
				<Skeleton className="h-12 w-full" />
			</output>
		)
	}
	if (state.kind === 'error') {
		return (
			<div className="flex flex-col items-start gap-2">
				<InlineAlert tone="danger" className="w-full">
					Impossible de charger les intégrations du client.
				</InlineAlert>
				<Button type="button" size="sm" variant="outline" onClick={onRetry}>
					Réessayer
				</Button>
			</div>
		)
	}
	if (state.instances.length === 0) {
		return (
			<InlineAlert tone="info">Ce client n’a aucune intégration.</InlineAlert>
		)
	}
	return (
		<ul className="flex max-h-72 flex-col divide-y overflow-y-auto rounded-md border bg-card">
			{state.instances.map((instance) => (
				<InstanceRow
					key={instance.id}
					instance={instance}
					value={choices[instance.id] ?? 'none'}
					disabled={disabled}
					onChange={(choice) => onChange(instance, choice)}
				/>
			))}
		</ul>
	)
}

export function InterveneCardDialog({
	open,
	cardTitle,
	instances,
	errorMessage,
	isPending,
	onRetry,
	onSubmit,
	onClose,
}: Props) {
	const [choices, setChoices] = useState<GrantChoices>({})
	const [instructions, setInstructions] = useState('')
	const [confirmed, setConfirmed] = useState(false)
	const list = instances.kind === 'ready' ? instances.instances : []
	const form: InterveneForm = { choices, instructions, confirmed }
	const length = instructionsLength(instructions)
	const askConfirmation = needsProductionConfirmation(list, choices)
	const canSubmit =
		instances.kind === 'ready' &&
		canSubmitIntervention(list, form) &&
		!isPending

	function choose(instance: Instance, choice: GrantChoice) {
		const next = { ...choices, [instance.id]: choice }
		setChoices(next)
		if (!needsProductionConfirmation(list, next)) setConfirmed(false)
	}

	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		if (canSubmit) onSubmit(form)
	}

	return (
		<Dialog
			open={open}
			onOpenChange={(next) => {
				if (!next && !isPending) onClose()
			}}
		>
			<DialogContent className="max-w-lg">
				<form
					onSubmit={handleSubmit}
					className="flex flex-col gap-4"
					noValidate
				>
					<DialogHeader>
						<DialogTitle>Confier à l’Intervenant</DialogTitle>
						<DialogDescription>
							L’Intervenant prend en charge la carte « {cardTitle} » sur les
							instances que vous autorisez.
						</DialogDescription>
					</DialogHeader>
					<InstancesField
						state={instances}
						choices={choices}
						disabled={isPending}
						onChange={choose}
						onRetry={onRetry}
					/>
					{askConfirmation ? (
						<label className="flex items-start gap-2 rounded-md border border-signal-danger/35 bg-signal-danger-soft px-3 py-2 text-[13px] font-medium text-signal-danger">
							<input
								type="checkbox"
								required
								className="mt-0.5 size-4 shrink-0 accent-signal-danger"
								checked={confirmed}
								disabled={isPending}
								onChange={(event) => setConfirmed(event.target.checked)}
							/>
							J’autorise l’écriture en production
						</label>
					) : null}
					<div className="flex flex-col gap-1.5">
						<Label htmlFor="intervene-instructions">Instructions</Label>
						<Textarea
							id="intervene-instructions"
							value={instructions}
							disabled={isPending}
							placeholder="Par exemple : vérifier dans Odoo staging, ne rien modifier en production"
							onChange={(event) => setInstructions(event.target.value)}
						/>
						<span
							className={cn(
								'self-end text-xs figures',
								length > INSTRUCTIONS_MAX_CHARS
									? 'text-signal-danger'
									: 'text-muted-foreground',
							)}
						>
							{length} / {INSTRUCTIONS_MAX_CHARS}
						</span>
					</div>
					<InlineAlert tone="info">
						L’Intervenant lit et modifie les instances choisies, dans la limite
						de ces autorisations. Chaque action est journalisée.
					</InlineAlert>
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
						<Button type="submit" disabled={!canSubmit}>
							{isPending ? 'Envoi…' : 'Confier'}
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	)
}
