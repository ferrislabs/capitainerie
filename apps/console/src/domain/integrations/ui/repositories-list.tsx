import { ExternalLink, Lock, Search } from 'lucide-react'
import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { Badge } from '#/components/ui/badge'
import { Input } from '#/components/ui/input'

interface Props {
	repositories: ReadonlyArray<Schemas.RepositoryResponse>
}

export function filterRepositories(
	repositories: ReadonlyArray<Schemas.RepositoryResponse>,
	query: string,
): ReadonlyArray<Schemas.RepositoryResponse> {
	const text = query.trim().toLowerCase()
	if (text === '') return repositories
	return repositories.filter((repository) =>
		repository.full_name.toLowerCase().includes(text),
	)
}

export function RepositoriesList({ repositories }: Props) {
	const [query, setQuery] = useState('')

	if (repositories.length === 0) {
		return (
			<p className="p-4 text-[13px] text-muted-foreground">
				L'application GitHub n'a accès à aucun dépôt. Ajoutes-en depuis les
				réglages de l'installation sur GitHub.
			</p>
		)
	}

	const shown = filterRepositories(repositories, query)

	return (
		<div className="flex flex-col">
			<div className="border-b p-3">
				<Input
					type="search"
					aria-label="Rechercher un dépôt"
					placeholder="Rechercher un dépôt"
					value={query}
					onChange={(event) => setQuery(event.target.value)}
					leading={<Search aria-hidden="true" />}
					clearable
				/>
			</div>
			{shown.length === 0 ? (
				<p className="p-4 text-[13px] text-muted-foreground">
					Aucun dépôt ne correspond à « {query.trim()} ».
				</p>
			) : (
				<ul className="divide-y">
					{shown.map((repository) => (
						<li
							key={repository.full_name}
							className="flex items-center justify-between gap-3 px-4 py-2.5"
						>
							<span className="min-w-0 truncate font-mono text-sm">
								{repository.full_name}
							</span>
							<span className="flex shrink-0 items-center gap-3">
								{repository.private ? (
									<Badge variant="muted">
										<Lock aria-hidden="true" className="size-3" />
										Privé
									</Badge>
								) : (
									<Badge variant="outline">Public</Badge>
								)}
								<a
									href={repository.html_url}
									target="_blank"
									rel="noreferrer"
									aria-label={`Ouvrir ${repository.full_name} sur GitHub`}
									className="text-muted-foreground hover:text-foreground"
								>
									<ExternalLink aria-hidden="true" className="size-4" />
								</a>
							</span>
						</li>
					))}
				</ul>
			)}
			<p className="border-t px-4 py-2 text-xs text-muted-foreground figures">
				{repositories.length} dépôt{repositories.length > 1 ? 's' : ''}{' '}
				accessible{repositories.length > 1 ? 's' : ''}
			</p>
		</div>
	)
}
