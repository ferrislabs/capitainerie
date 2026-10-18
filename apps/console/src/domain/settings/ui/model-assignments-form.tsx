import { ChevronDown } from 'lucide-react'
import type { FormEvent } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { PROVIDER_INFO } from '#/domain/settings/providers'
import {
	AGENT_ROLES,
	type AgentRole,
	type AssignmentDraft,
	catalogRef,
	compatibleModels,
	EXECUTOR_INFO,
	executorModels,
	groupByProvider,
	type ModelRef,
	modelRefKey,
	parseModelRefKey,
	ROLE_INFO,
} from '#/domain/settings/roles'

interface Props {
	models: ReadonlyArray<Schemas.ModelResponse>
	values: AssignmentDraft
	usesWorkerDefault: boolean
	isAdmin: boolean
	readOnlyMessage: string
	canSubmit: boolean
	isPending: boolean
	embeddingsChanged: boolean
	errorMessage?: string
	successMessage?: string
	onChange: (role: AgentRole, value: ModelRef) => void
	onExecutorChange: (model: string) => void
	onSubmit: () => void
}

function optionLabel(model: Schemas.ModelResponse): string {
	return model.display_name && model.display_name !== model.id
		? `${model.id} — ${model.display_name}`
		: model.id
}

export function ModelAssignmentsForm({
	models,
	values,
	usesWorkerDefault,
	isAdmin,
	readOnlyMessage,
	canSubmit,
	isPending,
	embeddingsChanged,
	errorMessage,
	successMessage,
	onChange,
	onExecutorChange,
	onSubmit,
}: Props) {
	const executorOptions = executorModels(models)
	const executor = values.executor ?? ''
	const executorKnown = executorOptions.some((model) => model.id === executor)
	const executorEmpty = executorOptions.length === 0 && executor === ''

	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		if (canSubmit) onSubmit()
	}

	return (
		<form onSubmit={handleSubmit} className="flex flex-col" noValidate>
			<div className="flex flex-col gap-2 border-b p-4">
				{usesWorkerDefault ? (
					<InlineAlert tone="info">
						Configuration par défaut du worker (Ollama local).
						{isAdmin
							? ' Choisissez un modèle pour chaque rôle pour la remplacer.'
							: null}
					</InlineAlert>
				) : null}
				{isAdmin ? null : (
					<p className="text-xs text-muted-foreground">{readOnlyMessage}</p>
				)}
				<p className="text-xs text-muted-foreground">
					Un changement s’applique aux runs déposés ensuite : un run déjà en
					file garde ses modèles.
				</p>
			</div>
			<ul className="flex flex-col divide-y">
				{AGENT_ROLES.map((role) => {
					const info = ROLE_INFO[role]
					const options = compatibleModels(models, role)
					const current = values[role]
					const currentKey = current ? modelRefKey(current) : ''
					const isKnown = options.some(
						(model) => modelRefKey(catalogRef(model)) === currentKey,
					)
					const selectId = `assignment-${role}`
					const empty = options.length === 0 && !current

					return (
						<li
							key={role}
							className="grid gap-2 px-4 py-3 md:grid-cols-[minmax(0,1fr)_minmax(16rem,22rem)] md:items-center md:gap-6"
						>
							<div className="flex min-w-0 flex-col gap-0.5">
								<label htmlFor={selectId} className="text-[13px] font-semibold">
									{info.label}
								</label>
								<p
									id={`${selectId}-description`}
									className="text-xs text-muted-foreground"
								>
									{info.description}
								</p>
							</div>
							<div className="relative">
								<select
									id={selectId}
									aria-describedby={`${selectId}-description`}
									className="h-8 w-full appearance-none truncate rounded-md border border-input bg-card pr-8 pl-3 font-mono text-xs text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-60"
									disabled={!isAdmin || isPending || empty}
									value={currentKey}
									onChange={(event) => {
										const ref = parseModelRefKey(event.target.value)
										if (ref) onChange(role, ref)
									}}
								>
									{currentKey === '' ? (
										<option value="" disabled>
											{empty
												? `Aucun modèle ${info.capability === 'chat' ? 'de chat' : 'd’embeddings'}`
												: 'Choisir un modèle…'}
										</option>
									) : null}
									{current && !isKnown ? (
										<option value={currentKey}>
											{current.model} ({PROVIDER_INFO[current.provider].label},
											indisponible)
										</option>
									) : null}
									{groupByProvider(options).map((group) => (
										<optgroup
											key={group.provider}
											label={PROVIDER_INFO[group.provider].label}
										>
											{group.items.map((model) => (
												<option
													key={modelRefKey(catalogRef(model))}
													value={modelRefKey(catalogRef(model))}
												>
													{optionLabel(model)}
												</option>
											))}
										</optgroup>
									))}
								</select>
								<ChevronDown
									aria-hidden="true"
									className="pointer-events-none absolute top-1/2 right-2.5 size-3.5 -translate-y-1/2 text-muted-foreground"
								/>
							</div>
						</li>
					)
				})}
				<li className="grid gap-2 px-4 py-3 md:grid-cols-[minmax(0,1fr)_minmax(16rem,22rem)] md:items-center md:gap-6">
					<div className="flex min-w-0 flex-col gap-0.5">
						<label
							htmlFor="assignment-executor"
							className="text-[13px] font-semibold"
						>
							{EXECUTOR_INFO.label}
						</label>
						<p
							id="assignment-executor-description"
							className="text-xs text-muted-foreground"
						>
							{EXECUTOR_INFO.description}
						</p>
					</div>
					<div className="relative">
						<select
							id="assignment-executor"
							aria-describedby="assignment-executor-description"
							className="h-8 w-full appearance-none truncate rounded-md border border-input bg-card pr-8 pl-3 font-mono text-xs text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-60"
							disabled={!isAdmin || isPending || executorEmpty}
							value={executor}
							onChange={(event) => {
								if (event.target.value) onExecutorChange(event.target.value)
							}}
						>
							{executor === '' ? (
								<option value="" disabled>
									{executorEmpty ? 'Aucun modèle Claude' : 'Choisir un modèle…'}
								</option>
							) : null}
							{executor !== '' && !executorKnown ? (
								<option value={executor}>{executor} (indisponible)</option>
							) : null}
							{executorOptions.map((model) => (
								<option key={model.id} value={model.id}>
									{optionLabel(model)}
								</option>
							))}
						</select>
						<ChevronDown
							aria-hidden="true"
							className="pointer-events-none absolute top-1/2 right-2.5 size-3.5 -translate-y-1/2 text-muted-foreground"
						/>
					</div>
				</li>
			</ul>
			{embeddingsChanged || errorMessage || successMessage || isAdmin ? (
				<div className="flex flex-col gap-3 border-t p-4">
					{embeddingsChanged ? (
						<InlineAlert tone="info">
							Les cartes seront réindexées au prochain run : l’index est propre
							à chaque modèle d’embeddings.
						</InlineAlert>
					) : null}
					{errorMessage ? (
						<InlineAlert tone="danger">{errorMessage}</InlineAlert>
					) : null}
					{successMessage ? (
						<InlineAlert tone="success">{successMessage}</InlineAlert>
					) : null}
					{isAdmin ? (
						<div className="flex justify-end">
							<Button type="submit" disabled={!canSubmit || isPending}>
								{isPending ? 'Enregistrement…' : 'Enregistrer'}
							</Button>
						</div>
					) : null}
				</div>
			) : null}
		</form>
	)
}
