import { Pencil, Plus, X } from 'lucide-react'
import { type FormEvent, type ReactNode, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { Checkbox } from '#/components/ui/checkbox'
import { Combobox, Select } from '#/components/ui/combobox'
import { DateInput } from '#/components/ui/date-input'
import { Field } from '#/components/ui/field'
import { Input } from '#/components/ui/input'
import { Textarea } from '#/components/ui/textarea'
import {
	criteriaProgress,
	ESTIMATE_LABELS,
	ESTIMATES,
	PRIORITIES,
	PRIORITY_LABELS,
} from '#/domain/kanban/card-details'
import {
	addCriterion,
	addLabel,
	type CardPatch,
	MAX_CRITERIA,
	MAX_CRITERION_CHARS,
	MAX_LABEL_CHARS,
	MAX_LABELS,
	removeCriterion,
	removeLabel,
	renameCriterion,
	toggleCriterion,
} from '#/domain/kanban/card-patch'

const LABEL_CLASS = 'text-xs font-normal text-muted-foreground'

interface EditFormProps {
	label: string
	value: string
	multiline?: boolean
	required?: boolean
	maxLength?: number
	disabled: boolean
	onSave: (value: string) => void
	onCancel: () => void
}

function EditForm({
	label,
	value,
	multiline = false,
	required = false,
	maxLength,
	disabled,
	onSave,
	onCancel,
}: EditFormProps) {
	const [draft, setDraft] = useState(value)
	const blank = draft.trim() === ''
	const unchanged = draft.trim() === value.trim()

	function submit(event: FormEvent) {
		event.preventDefault()
		if ((required && blank) || unchanged) return
		onSave(draft.trim())
	}

	return (
		<form onSubmit={submit} className="flex flex-col gap-2">
			{multiline ? (
				<Textarea
					aria-label={label}
					value={draft}
					maxLength={maxLength}
					rows={5}
					maxRows={14}
					onChange={(event) => setDraft(event.target.value)}
				/>
			) : (
				<Input
					aria-label={label}
					value={draft}
					maxLength={maxLength}
					onChange={(event) => setDraft(event.target.value)}
				/>
			)}
			<div className="flex gap-2">
				<Button
					type="submit"
					size="sm"
					disabled={disabled || unchanged || (required && blank)}
				>
					Enregistrer
				</Button>
				<Button type="button" size="sm" variant="ghost" onClick={onCancel}>
					Annuler
				</Button>
			</div>
		</form>
	)
}

interface TitleProps {
	title: string
	disabled: boolean
	onSave: (title: string) => void
	children: (title: string) => ReactNode
}

export function EditableTitle({
	title,
	disabled,
	onSave,
	children,
}: TitleProps) {
	const [editing, setEditing] = useState(false)
	if (editing) {
		return (
			<EditForm
				label="Titre"
				value={title}
				required
				maxLength={200}
				disabled={disabled}
				onSave={(value) => {
					setEditing(false)
					onSave(value)
				}}
				onCancel={() => setEditing(false)}
			/>
		)
	}
	return (
		<div className="flex items-start gap-1">
			{children(title)}
			<Button
				type="button"
				variant="ghost"
				size="icon-sm"
				aria-label="Modifier le titre"
				disabled={disabled}
				onClick={() => setEditing(true)}
			>
				<Pencil aria-hidden="true" className="size-3.5" />
			</Button>
		</div>
	)
}

interface DescriptionProps {
	description: string
	disabled: boolean
	onSave: (description: string) => void
}

export function EditableDescription({
	description,
	disabled,
	onSave,
}: DescriptionProps) {
	const [editing, setEditing] = useState(false)
	return (
		<section className="flex flex-col gap-2">
			<div className="flex items-center justify-between gap-2">
				<h3 className="text-[15px] font-semibold">Description</h3>
				{editing ? null : (
					<Button
						type="button"
						variant="ghost"
						size="sm"
						disabled={disabled}
						onClick={() => setEditing(true)}
					>
						<Pencil aria-hidden="true" className="size-3.5" />
						Modifier la description
					</Button>
				)}
			</div>
			{editing ? (
				<EditForm
					label="Description"
					value={description}
					multiline
					maxLength={10_000}
					disabled={disabled}
					onSave={(value) => {
						setEditing(false)
						onSave(value)
					}}
					onCancel={() => setEditing(false)}
				/>
			) : description ? (
				<p className="text-[13px] leading-relaxed break-words whitespace-pre-wrap">
					{description}
				</p>
			) : (
				<p className="text-[13px] text-muted-foreground">Aucune description.</p>
			)}
		</section>
	)
}

function LabelsEditor({
	labels,
	disabled,
	onEdit,
}: {
	labels: ReadonlyArray<string>
	disabled: boolean
	onEdit: (patch: CardPatch) => void
}) {
	const [draft, setDraft] = useState('')

	function submit(event: FormEvent) {
		event.preventDefault()
		const next = addLabel(labels, draft)
		if (!next) return
		setDraft('')
		onEdit({ labels: next })
	}

	return (
		<div className="flex flex-col gap-1.5 text-xs text-muted-foreground sm:col-span-2">
			<span id="card-labels">Étiquettes</span>
			<ul aria-labelledby="card-labels" className="flex flex-wrap gap-1.5">
				{labels.map((label) => (
					<li key={label}>
						<Badge variant="outline" className="gap-1 pr-0.5">
							{label}
							<button
								type="button"
								aria-label={`Retirer l’étiquette ${label}`}
								disabled={disabled}
								className="inline-flex size-4 items-center justify-center rounded-sm hover:bg-muted disabled:opacity-50"
								onClick={() => onEdit({ labels: removeLabel(labels, label) })}
							>
								<X aria-hidden="true" className="size-3" />
							</button>
						</Badge>
					</li>
				))}
			</ul>
			{labels.length < MAX_LABELS ? (
				<form onSubmit={submit} className="flex gap-2">
					<Input
						aria-label="Ajouter une étiquette"
						placeholder="Nouvelle étiquette"
						value={draft}
						maxLength={MAX_LABEL_CHARS}
						disabled={disabled}
						onChange={(event) => setDraft(event.target.value)}
					/>
					<Button
						aria-label="Ajouter l’étiquette"
						type="submit"
						size="sm"
						variant="outline"
						disabled={disabled || draft.trim() === ''}
					>
						Ajouter
					</Button>
				</form>
			) : null}
		</div>
	)
}

function CriterionRow({
	criterion,
	disabled,
	onToggle,
	onRename,
	onRemove,
}: {
	criterion: Schemas.CriterionDto
	disabled: boolean
	onToggle: () => void
	onRename: (text: string) => void
	onRemove: () => void
}) {
	const [editing, setEditing] = useState(false)
	if (editing) {
		return (
			<li>
				<EditForm
					label="Critère"
					value={criterion.text}
					required
					maxLength={MAX_CRITERION_CHARS}
					disabled={disabled}
					onSave={(text) => {
						setEditing(false)
						onRename(text)
					}}
					onCancel={() => setEditing(false)}
				/>
			</li>
		)
	}
	return (
		<li className="flex items-start gap-2">
			<Checkbox
				aria-label={criterion.text}
				checked={criterion.done}
				disabled={disabled}
				className="mt-0.5"
				onCheckedChange={onToggle}
			/>
			<span
				className={
					criterion.done
						? 'flex-1 text-[13px] break-words text-muted-foreground line-through'
						: 'flex-1 text-[13px] break-words'
				}
			>
				{criterion.text}
			</span>
			<Button
				type="button"
				variant="ghost"
				size="icon-sm"
				aria-label={`Modifier le critère ${criterion.text}`}
				disabled={disabled}
				onClick={() => setEditing(true)}
			>
				<Pencil aria-hidden="true" className="size-3.5" />
			</Button>
			<Button
				type="button"
				variant="ghost"
				size="icon-sm"
				aria-label={`Supprimer le critère ${criterion.text}`}
				disabled={disabled}
				onClick={onRemove}
			>
				<X aria-hidden="true" className="size-3.5" />
			</Button>
		</li>
	)
}

function CriteriaEditor({
	criteria,
	disabled,
	onEdit,
}: {
	criteria: ReadonlyArray<Schemas.CriterionDto>
	disabled: boolean
	onEdit: (patch: CardPatch) => void
}) {
	const [draft, setDraft] = useState('')
	const progress = criteriaProgress(criteria)

	function submit(event: FormEvent) {
		event.preventDefault()
		const next = addCriterion(criteria, draft)
		if (!next) return
		setDraft('')
		onEdit({ acceptance_criteria: next })
	}

	return (
		<section className="flex flex-col gap-2">
			<div className="flex items-baseline justify-between gap-2">
				<h3 className="text-[15px] font-semibold">Critères d’acceptation</h3>
				{progress ? (
					<span className="text-xs text-muted-foreground figures">
						{progress.done}/{progress.total}
					</span>
				) : null}
			</div>
			{criteria.length === 0 ? (
				<p className="text-[13px] text-muted-foreground">
					Aucun critère d’acceptation.
				</p>
			) : (
				<ul className="flex flex-col gap-1.5">
					{criteria.map((criterion, index) => (
						<CriterionRow
							// biome-ignore lint/suspicious/noArrayIndexKey: criteria have no identity and are replaced as a whole
							key={`${index}-${criterion.text}`}
							criterion={criterion}
							disabled={disabled}
							onToggle={() =>
								onEdit({
									acceptance_criteria: toggleCriterion(criteria, index),
								})
							}
							onRename={(text) =>
								onEdit({
									acceptance_criteria: renameCriterion(criteria, index, text),
								})
							}
							onRemove={() =>
								onEdit({
									acceptance_criteria: removeCriterion(criteria, index),
								})
							}
						/>
					))}
				</ul>
			)}
			{criteria.length < MAX_CRITERIA ? (
				<form onSubmit={submit} className="flex gap-2">
					<Input
						aria-label="Nouveau critère"
						placeholder="Ce qui doit être vrai une fois la carte terminée"
						value={draft}
						maxLength={MAX_CRITERION_CHARS}
						disabled={disabled}
						onChange={(event) => setDraft(event.target.value)}
					/>
					<Button
						aria-label="Ajouter le critère"
						type="submit"
						size="sm"
						variant="outline"
						disabled={disabled || draft.trim() === ''}
					>
						<Plus aria-hidden="true" className="size-3.5" />
						Ajouter
					</Button>
				</form>
			) : null}
		</section>
	)
}

interface Props {
	card: Schemas.CardResponse
	members: ReadonlyArray<Schemas.MemberResponse>
	membersReady: boolean
	disabled: boolean
	onEdit: (patch: CardPatch) => void
}

export function CardFields({
	card,
	members,
	membersReady,
	disabled,
	onEdit,
}: Props) {
	const assignee = card.assignee_subject
	const known = members.some((member) => member.subject === assignee)

	return (
		<>
			<section className="flex flex-col gap-2">
				<h3 className="text-[15px] font-semibold">Détails</h3>
				<div className="grid gap-3 sm:grid-cols-2">
					<Field
						label="Priorité"
						id="card-priority"
						labelClassName={LABEL_CLASS}
					>
						<Select
							size="sm"
							disabled={disabled}
							items={PRIORITIES.map((priority) => ({
								value: priority,
								label: PRIORITY_LABELS[priority],
							}))}
							value={card.priority}
							onValueChange={(value) =>
								onEdit({ priority: value as Schemas.PriorityDto })
							}
						/>
					</Field>
					<Field
						label="Estimation"
						id="card-estimate"
						labelClassName={LABEL_CLASS}
					>
						<Select
							size="sm"
							disabled={disabled}
							items={ESTIMATES.map((estimate) => ({
								value: estimate,
								label: ESTIMATE_LABELS[estimate],
							}))}
							value={card.estimate ?? ''}
							noneLabel="Aucune"
							onValueChange={(value) =>
								onEdit({
									estimate:
										value === '' ? null : (value as Schemas.EstimateDto),
								})
							}
						/>
					</Field>
					<Field
						label="Échéance"
						id="card-due-date"
						labelClassName={LABEL_CLASS}
					>
						<DateInput
							size="sm"
							disabled={disabled}
							value={card.due_date ?? ''}
							onChange={(event) =>
								onEdit({ due_date: event.target.value || null })
							}
						/>
					</Field>
					<Field
						label="Responsable"
						id="card-assignee"
						labelClassName={LABEL_CLASS}
					>
						<Combobox
							size="sm"
							disabled={disabled || !membersReady}
							items={[
								...(assignee && !known
									? [{ value: assignee, label: assignee }]
									: []),
								...members.map((member) => ({
									value: member.subject,
									label: member.name,
								})),
							]}
							value={assignee ?? ''}
							noneLabel="Aucun"
							onValueChange={(value) =>
								onEdit({ assignee_subject: value || null })
							}
						/>
					</Field>
					<LabelsEditor
						labels={card.labels}
						disabled={disabled}
						onEdit={onEdit}
					/>
				</div>
			</section>
			<CriteriaEditor
				criteria={card.acceptance_criteria}
				disabled={disabled}
				onEdit={onEdit}
			/>
		</>
	)
}
