import type { FormEvent } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Combobox } from '#/components/ui/combobox'
import type { ComboboxItem } from '#/components/ui/combobox-model'
import { PROVIDER_INFO } from '#/domain/settings/providers'
import {
	AGENT_ROLES,
	type AgentRole,
	type AssignmentDraft,
	CAPTAIN_ROLE,
	catalogRef,
	compatibleModels,
	EXECUTOR_INFO,
	executorModels,
	groupByProvider,
	INTEGRATOR_ROLE,
	type ModelRef,
	modelRefKey,
	parseModelRefKey,
	READER_ROLE,
	ROLE_INFO,
	type RoleInfo,
	TRANSCRIBER_ROLE,
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
	onTranscriberChange: (value: ModelRef | undefined) => void
	onReaderChange: (value: ModelRef | undefined) => void
	onCaptainChange: (value: ModelRef | undefined) => void
	onIntegratorChange: (value: ModelRef | undefined) => void
	onExecutorChange: (model: string) => void
	onSubmit: () => void
}

function optionLabel(model: Schemas.ModelResponse): string {
	return model.display_name && model.display_name !== model.id
		? `${model.id} — ${model.display_name}`
		: model.id
}

function unavailableLabel(ref: ModelRef): string {
	return `${ref.model} (${PROVIDER_INFO[ref.provider].label}, indisponible)`
}

function modelItems(
	options: ReadonlyArray<Schemas.ModelResponse>,
	unavailable: ModelRef | undefined,
): ComboboxItem[] {
	const groups = groupByProvider(options).map((group) => ({
		label: PROVIDER_INFO[group.provider].label,
		options: group.items.map((model) => ({
			value: modelRefKey(catalogRef(model)),
			label: optionLabel(model),
		})),
	}))
	return unavailable
		? [
				{
					value: modelRefKey(unavailable),
					label: unavailableLabel(unavailable),
				},
				...groups,
			]
		: groups
}

interface OptionalRoleRowProps {
	id: string
	info: RoleInfo
	options: ReadonlyArray<Schemas.ModelResponse>
	value: ModelRef | undefined
	noModelLabel: string
	noneLabel: string
	disabled: boolean
	onChange: (value: ModelRef | undefined) => void
}

function OptionalRoleRow({
	id,
	info,
	options,
	value,
	noModelLabel,
	noneLabel,
	disabled,
	onChange,
}: OptionalRoleRowProps) {
	const key = value ? modelRefKey(value) : ''
	const known = options.some((model) => modelRefKey(catalogRef(model)) === key)
	const empty = options.length === 0 && !value

	return (
		<li className="grid gap-2 px-4 py-3 md:grid-cols-[minmax(0,1fr)_minmax(16rem,22rem)] md:items-center md:gap-6">
			<div className="flex min-w-0 flex-col gap-0.5">
				<label htmlFor={id} className="text-[13px] font-semibold">
					{info.label}
				</label>
				<p id={`${id}-description`} className="text-xs text-muted-foreground">
					{info.description}
				</p>
			</div>
			<Combobox
				id={id}
				aria-describedby={`${id}-description`}
				className="font-mono text-xs"
				disabled={disabled || empty}
				items={modelItems(options, value && !known ? value : undefined)}
				value={key}
				onValueChange={(next) => onChange(parseModelRefKey(next))}
				noneLabel={empty ? noModelLabel : noneLabel}
			/>
		</li>
	)
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
	onTranscriberChange,
	onReaderChange,
	onCaptainChange,
	onIntegratorChange,
	onExecutorChange,
	onSubmit,
}: Props) {
	const executorOptions = executorModels(models)
	const executor = values.executor ?? ''
	const executorKnown = executorOptions.some((model) => model.id === executor)
	const executorEmpty = executorOptions.length === 0 && executor === ''
	const transcriberInfo = ROLE_INFO[TRANSCRIBER_ROLE]
	const transcriberOptions = compatibleModels(models, TRANSCRIBER_ROLE)
	const readerInfo = ROLE_INFO[READER_ROLE]
	const readerOptions = compatibleModels(models, READER_ROLE)
	const captainInfo = ROLE_INFO[CAPTAIN_ROLE]
	const captainOptions = compatibleModels(models, CAPTAIN_ROLE)
	const integratorInfo = ROLE_INFO[INTEGRATOR_ROLE]
	const integratorOptions = compatibleModels(models, INTEGRATOR_ROLE)

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
							<Combobox
								id={selectId}
								aria-describedby={`${selectId}-description`}
								className="font-mono text-xs"
								disabled={!isAdmin || isPending || empty}
								items={modelItems(
									options,
									current && !isKnown ? current : undefined,
								)}
								value={currentKey}
								onValueChange={(next) => {
									const ref = parseModelRefKey(next)
									if (ref) onChange(role, ref)
								}}
								placeholder={
									empty
										? `Aucun modèle ${info.capability === 'chat' ? 'de chat' : 'd’embeddings'}`
										: 'Choisir un modèle…'
								}
							/>
						</li>
					)
				})}
				<OptionalRoleRow
					id="assignment-transcriber"
					info={transcriberInfo}
					options={transcriberOptions}
					value={values[TRANSCRIBER_ROLE]}
					noModelLabel="Aucun modèle de transcription"
					noneLabel="Aucun transcripteur"
					disabled={!isAdmin || isPending}
					onChange={onTranscriberChange}
				/>
				<OptionalRoleRow
					id="assignment-reader"
					info={readerInfo}
					options={readerOptions}
					value={values[READER_ROLE]}
					noModelLabel="Aucun modèle OCR"
					noneLabel="Aucun lecteur de PDF"
					disabled={!isAdmin || isPending}
					onChange={onReaderChange}
				/>
				<OptionalRoleRow
					id="assignment-captain"
					info={captainInfo}
					options={captainOptions}
					value={values[CAPTAIN_ROLE]}
					noModelLabel="Aucun modèle de chat"
					noneLabel="Le modèle du Rédacteur"
					disabled={!isAdmin || isPending}
					onChange={onCaptainChange}
				/>
				<OptionalRoleRow
					id="assignment-integrator"
					info={integratorInfo}
					options={integratorOptions}
					value={values[INTEGRATOR_ROLE]}
					noModelLabel="Aucun modèle de chat"
					noneLabel="Le modèle du Rédacteur"
					disabled={!isAdmin || isPending}
					onChange={onIntegratorChange}
				/>
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
					<Combobox
						id="assignment-executor"
						aria-describedby="assignment-executor-description"
						className="font-mono text-xs"
						disabled={!isAdmin || isPending || executorEmpty}
						items={[
							...(executor !== '' && !executorKnown
								? [{ value: executor, label: `${executor} (indisponible)` }]
								: []),
							...executorOptions.map((model) => ({
								value: model.id,
								label: optionLabel(model),
							})),
						]}
						value={executor}
						onValueChange={(next) => {
							if (next) onExecutorChange(next)
						}}
						placeholder={
							executorEmpty ? 'Aucun modèle Claude' : 'Choisir un modèle…'
						}
					/>
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
