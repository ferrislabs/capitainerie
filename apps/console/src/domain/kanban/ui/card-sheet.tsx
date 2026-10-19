import { Link } from '@tanstack/react-router'
import { CircleAlert } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import {
	Sheet,
	SheetBody,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
} from '#/components/ui/sheet'
import { Skeleton } from '#/components/ui/skeleton'
import { canDrop } from '#/domain/kanban/can-drop'
import type { CardPatch } from '#/domain/kanban/card-patch'
import {
	agentName,
	COLUMN_LABELS,
	COLUMNS,
	type Column,
} from '#/domain/kanban/columns'
import type { LinkPatch } from '#/domain/kanban/link-patch'
import { CardComments } from '#/domain/kanban/ui/card-comments'
import {
	CardFields,
	EditableDescription,
	EditableTitle,
} from '#/domain/kanban/ui/card-fields'
import { CardLinks } from '#/domain/kanban/ui/card-links'
import { CardRuns } from '#/domain/kanban/ui/card-runs'
import { LinkChips } from '#/domain/kanban/ui/link-chips'

export type CardSheetStatus = 'loading' | 'ready' | 'not_found' | 'error'

interface Props {
	open: boolean
	status: CardSheetStatus
	card: Schemas.CardResponse | undefined
	comments: ReadonlyArray<Schemas.CommentResponse>
	runs: ReadonlyArray<Schemas.RunResponse>
	currentSubject: string | undefined
	isMoving: boolean
	moveError: string | undefined
	draft: string
	isCommenting: boolean
	commentError: string | undefined
	runExtra: (run: Schemas.RunResponse) => ReactNode
	clients?: ReadonlyArray<Schemas.ClientResponse>
	projects?: ReadonlyArray<Schemas.ProjectResponse>
	isLinking?: boolean
	linksReady?: boolean
	linkError?: string | undefined
	onLink?: (patch: LinkPatch) => void
	members?: ReadonlyArray<Schemas.MemberResponse>
	membersReady?: boolean
	isEditing?: boolean
	editError?: string | undefined
	onEdit?: (patch: CardPatch) => void
	onClose: () => void
	onMove: (to: Column) => void
	onExecute: (card: Schemas.CardResponse) => void
	onDraftChange: (value: string) => void
	onSubmitComment: () => void
}

function Notice({ children }: { children: ReactNode }) {
	return (
		<p className="flex items-start gap-2 text-[13px] text-muted-foreground">
			<CircleAlert aria-hidden="true" className="mt-px size-4 shrink-0" />
			{children}
		</p>
	)
}

function LoadingBody() {
	return (
		<SheetBody>
			<output className="flex flex-col gap-3">
				<span className="sr-only">Chargement…</span>
				<Skeleton className="h-4 w-full" />
				<Skeleton className="h-4 w-5/6" />
				<Skeleton className="h-24 w-full rounded-md" />
			</output>
		</SheetBody>
	)
}

export function CardSheet({
	open,
	status,
	card,
	comments,
	runs,
	currentSubject,
	isMoving,
	moveError,
	draft,
	isCommenting,
	commentError,
	runExtra,
	clients = [],
	projects = [],
	isLinking = false,
	linksReady = false,
	linkError,
	onLink,
	members = [],
	membersReady = false,
	isEditing = false,
	editError,
	onEdit,
	onClose,
	onMove,
	onExecute,
	onDraftChange,
	onSubmitComment,
}: Props) {
	const agent = card?.created_by_agent

	return (
		<Sheet
			open={open}
			onOpenChange={(next) => {
				if (!next) onClose()
			}}
		>
			<SheetContent aria-describedby={undefined}>
				<SheetHeader>
					{card && onEdit ? (
						<EditableTitle
							title={card.title}
							disabled={isEditing}
							onSave={(title) => onEdit({ title })}
						>
							{(title) => <SheetTitle>{title}</SheetTitle>}
						</EditableTitle>
					) : (
						<SheetTitle>{card?.title ?? 'Carte'}</SheetTitle>
					)}
					{card ? (
						<div className="flex flex-wrap items-center gap-2">
							<Badge
								variant={card.column === 'proposed' ? 'attention' : 'muted'}
							>
								{COLUMN_LABELS[card.column]}
							</Badge>
							{agent ? (
								<Badge variant="secondary" className="font-mono text-[11px]">
									{agentName(agent)}
								</Badge>
							) : null}
							<LinkChips client={card.client} project={card.project} />
							<label className="ml-auto flex items-center gap-2 text-xs text-muted-foreground">
								<span className="sr-only sm:not-sr-only">Déplacer vers…</span>
								<select
									aria-label="Déplacer vers…"
									className="h-7 rounded-md border border-input bg-card pr-2 pl-2 text-xs text-foreground focus-visible:border-ring focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/25 disabled:opacity-50"
									disabled={isMoving}
									value=""
									onChange={(event) => {
										if (event.target.value) onMove(event.target.value as Column)
									}}
								>
									<option value="">Choisir…</option>
									{COLUMNS.filter(
										(column) =>
											column !== card.column && canDrop(card.column, column),
									).map((column) => (
										<option key={column} value={column}>
											{COLUMN_LABELS[column]}
										</option>
									))}
								</select>
							</label>
						</div>
					) : null}
					{moveError ? (
						<InlineAlert tone="danger">{moveError}</InlineAlert>
					) : null}
					<SheetDescription className="sr-only">
						Détail de la carte, ses runs et ses commentaires.
					</SheetDescription>
				</SheetHeader>
				{status === 'loading' && !card ? <LoadingBody /> : null}
				{status === 'not_found' ? (
					<SheetBody>
						<Notice>Cette carte n’existe pas dans votre organisation.</Notice>
					</SheetBody>
				) : null}
				{status === 'error' && !card ? (
					<SheetBody>
						<InlineAlert tone="danger">
							Impossible de charger la carte.
						</InlineAlert>
					</SheetBody>
				) : null}
				{card ? (
					<SheetBody>
						{editError ? (
							<InlineAlert tone="danger">{editError}</InlineAlert>
						) : null}
						{onEdit ? (
							<EditableDescription
								description={card.description}
								disabled={isEditing}
								onSave={(description) => onEdit({ description })}
							/>
						) : (
							<section className="flex flex-col gap-2">
								<h3 className="text-[15px] font-semibold">Description</h3>
								{card.description ? (
									<p className="text-[13px] leading-relaxed break-words whitespace-pre-wrap">
										{card.description}
									</p>
								) : (
									<p className="text-[13px] text-muted-foreground">
										Aucune description.
									</p>
								)}
							</section>
						)}
						{onEdit ? (
							<CardFields
								card={card}
								members={members}
								membersReady={membersReady}
								disabled={isEditing}
								onEdit={onEdit}
							/>
						) : null}
						{card.provenance ? (
							<figure className="flex flex-col gap-1 rounded-md bg-muted/70 px-3 py-2">
								<figcaption className="text-[11px] font-medium text-muted-foreground">
									extrait du transcript
								</figcaption>
								<blockquote className="text-[13px] leading-relaxed italic">
									« {card.provenance.quote} »
								</blockquote>
								{card.created_by_run ? (
									<Link
										to="/runs/$runId"
										params={{ runId: card.created_by_run }}
										className="w-fit text-xs font-medium text-link hover:underline"
									>
										Voir le run d’origine
									</Link>
								) : null}
							</figure>
						) : null}
						{onLink ? (
							<CardLinks
								card={card}
								clients={clients}
								projects={projects}
								isPending={isLinking}
								ready={linksReady}
								errorMessage={linkError}
								onLink={onLink}
							/>
						) : null}
						{card.column === 'todo' ? (
							<Button
								type="button"
								variant="outline"
								size="sm"
								className="w-fit"
								disabled={isMoving}
								onClick={() => onExecute(card)}
							>
								Confier à Claude Code
							</Button>
						) : null}
						{status === 'ready' ? (
							<>
								<CardRuns runs={runs} runExtra={runExtra} />
								<CardComments
									comments={comments}
									currentSubject={currentSubject}
									draft={draft}
									isPending={isCommenting}
									errorMessage={commentError}
									onDraftChange={onDraftChange}
									onSubmit={onSubmitComment}
								/>
							</>
						) : status === 'error' ? (
							<InlineAlert tone="danger">
								Impossible de charger les commentaires et les runs.
							</InlineAlert>
						) : (
							<LoadingBody />
						)}
					</SheetBody>
				) : null}
			</SheetContent>
		</Sheet>
	)
}
