import { Plus, Search, Sparkles } from 'lucide-react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { HumanAvatar } from '#/domain/audit/ui/avatar'
import { Panel, PanelEmpty, Row, Rows, RowTitle } from '#/domain/audit/ui/rows'
import { Sev } from '#/domain/audit/ui/sev'
import { type Skill, sourceLabel } from '#/domain/skills/model'

interface Props {
	skills: ReadonlyArray<Skill>
	total: number
	loading: boolean
	error: boolean
	query: string
	showArchived: boolean
	notice: string | null
	onQueryChange: (query: string) => void
	onToggleArchived: () => void
	onRetry: () => void
	onCreate: () => void
	onOpen: (skill: Skill) => void
}

function describe(total: number, archived: boolean): string {
	const noun = total > 1 ? 'compétences' : 'compétence'
	return archived
		? `${total} ${noun} ${total > 1 ? 'archivées' : 'archivée'}`
		: `${total} ${noun} disponible${total > 1 ? 's' : ''} pour l'organisation`
}

export function SkillsView({
	skills,
	total,
	loading,
	error,
	query,
	showArchived,
	notice,
	onQueryChange,
	onToggleArchived,
	onRetry,
	onCreate,
	onOpen,
}: Props) {
	return (
		<Page>
			<PageHeader
				title="Compétences"
				description={
					loading || error ? undefined : describe(total, showArchived)
				}
				actions={
					<>
						<Button
							variant="outline"
							aria-pressed={showArchived}
							onClick={onToggleArchived}
						>
							{showArchived ? 'Voir les actives' : 'Voir les archivées'}
						</Button>
						<Button onClick={onCreate}>
							<Plus aria-hidden="true" />
							Nouvelle
						</Button>
					</>
				}
			/>
			<Notice message={notice} className="mb-3" />
			<label className="mb-3 flex max-w-[360px] items-center gap-2 rounded-[6px] border bg-background px-[9px] py-1.5 focus-within:border-primary">
				<Search aria-hidden="true" className="size-4 text-muted-foreground" />
				<input
					type="search"
					aria-label="Rechercher une compétence"
					placeholder="Rechercher…"
					value={query}
					onChange={(event) => onQueryChange(event.target.value)}
					className="min-w-0 flex-1 bg-transparent text-[13px] outline-none"
				/>
			</label>
			{error ? (
				<div className="grid gap-2">
					<InlineAlert tone="danger">
						Impossible de charger les compétences.
					</InlineAlert>
					<div>
						<Button variant="outline" onClick={onRetry}>
							Réessayer
						</Button>
					</div>
				</div>
			) : loading ? (
				<output className="m-0 text-muted-foreground">
					Chargement des compétences…
				</output>
			) : (
				<Panel>
					{skills.length === 0 ? (
						<PanelEmpty
							icon={Sparkles}
							title={
								query.trim()
									? 'Aucune compétence ne correspond'
									: showArchived
										? 'Aucune compétence archivée'
										: 'Aucune compétence'
							}
						>
							{query.trim() || showArchived
								? null
								: 'Crée une compétence pour la donner à tes agents.'}
						</PanelEmpty>
					) : (
						<Rows>
							{skills.map((s) => (
								<Row key={s.id} className="p-0">
									<button
										type="button"
										onClick={() => onOpen(s)}
										aria-label={`Ouvrir ${s.name}`}
										className="flex min-h-9 w-full min-w-0 items-center gap-2.5 px-3 py-1.5 text-left"
									>
										<HumanAvatar
											initials={s.name.charAt(0).toUpperCase()}
											size={26}
										/>
										<RowTitle
											title={
												<>
													{s.name}{' '}
													<span className="font-mono text-xs font-normal text-muted-foreground">
														{s.slug}
													</span>
												</>
											}
											sub={s.description || undefined}
										/>
										<span className="text-xs whitespace-nowrap text-muted-foreground">
											{s.used_by} {s.used_by > 1 ? 'agents' : 'agent'}
										</span>
										<Sev tone="info">{sourceLabel(s.source)}</Sev>
									</button>
								</Row>
							))}
						</Rows>
					)}
				</Panel>
			)}
		</Page>
	)
}
