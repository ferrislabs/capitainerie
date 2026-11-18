import { Link } from '@tanstack/react-router'
import { X } from 'lucide-react'
import { Button } from '#/components/ui/button'
import { Combobox } from '#/components/ui/combobox'
import { DetailSection } from '#/domain/agent-team/ui/panel'
import { DETAIL_ROW, Empty, Rows, Ttl } from '#/domain/agent-team/ui/rows'
import { Sev } from '#/domain/agent-team/ui/sev'
import type { RepositoriesState } from '#/domain/integrations/hooks/use-github'

export const MAX_REPOSITORY_GRANTS = 50

interface Props {
	granted: ReadonlyArray<string>
	state: RepositoriesState
	busy: boolean
	onAdd: (repository: string) => void
	onRemove: (repository: string) => void
}

const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase()

export function RepositoryGrantsSection({
	granted,
	state,
	busy,
	onAdd,
	onRemove,
}: Props) {
	const reachable = state.kind === 'ready' ? state.repositories : []
	const candidates = reachable.filter(
		(repository) => !granted.some((name) => same(name, repository.full_name)),
	)
	const full = granted.length >= MAX_REPOSITORY_GRANTS

	return (
		<DetailSection
			title="Dépôts GitHub"
			flush
			actions={<Link to="/settings/integrations/github">Gérer GitHub</Link>}
		>
			<p className="m-0 border-b px-6 py-5 text-muted-foreground">
				L'agent ne peut travailler que sur les dépôts listés ici. Sans
				écritures, il les audite en lecture seule ; avec, il pousse une branche
				et ouvre une pull request brouillon.
			</p>
			{state.kind === 'not_connected' ? (
				<p className="m-0 px-6 py-5 text-muted-foreground">
					GitHub n'est pas connecté à l'organisation.{' '}
					<Link to="/settings/integrations/github" className="text-primary">
						Connecter GitHub
					</Link>
				</p>
			) : null}
			{state.kind === 'error' ? (
				<p role="alert" className="m-0 px-6 py-5 text-signal-danger">
					Impossible de charger les dépôts GitHub.
				</p>
			) : null}
			{granted.length === 0 ? (
				<Empty title="Aucun dépôt">
					Ajoute un dépôt pour que l'agent puisse travailler sur du code.
				</Empty>
			) : (
				<Rows>
					<ul aria-label="Dépôts autorisés" className="m-0 list-none p-0">
						{granted.map((name) => {
							const known = reachable.find((r) => same(r.full_name, name))
							const unreachable = state.kind === 'ready' && known === undefined
							return (
								<li key={name} className={DETAIL_ROW}>
									<Ttl
										title={name}
										titleMono
										sub={
											unreachable
												? "L'application GitHub n'a plus accès à ce dépôt"
												: undefined
										}
									/>
									{known ? (
										<Sev tone="info">{known.private ? 'Privé' : 'Public'}</Sev>
									) : null}
									<Button
										type="button"
										size="icon-sm"
										variant="ghost"
										aria-label={`Retirer ${name}`}
										disabled={busy}
										onClick={() => onRemove(name)}
									>
										<X aria-hidden="true" />
									</Button>
								</li>
							)
						})}
					</ul>
				</Rows>
			)}
			{state.kind === 'ready' || state.kind === 'loading' ? (
				<div className="border-t px-6 py-5">
					<Combobox
						aria-label="Ajouter un dépôt"
						className="font-mono text-xs"
						items={candidates.map((repository) => ({
							value: repository.full_name,
							label: repository.full_name,
						}))}
						value=""
						loading={state.kind === 'loading'}
						disabled={busy || full}
						onValueChange={(value) => {
							if (value) onAdd(value)
						}}
						placeholder={
							full
								? `Limite de ${MAX_REPOSITORY_GRANTS} dépôts atteinte`
								: 'Ajouter un dépôt…'
						}
						emptyText={
							reachable.length === 0
								? "L'application GitHub n'a accès à aucun dépôt"
								: 'Aucun dépôt ne correspond'
						}
					/>
				</div>
			) : null}
		</DetailSection>
	)
}
