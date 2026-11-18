import { Link, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { Page } from '#/components/layout/page'
import { Button } from '#/components/ui/button'
import { useSourceFormOptions } from '#/domain/sources/feature/use-form-options'
import { useCreateSource } from '#/domain/sources/hooks/use-sources'
import {
	createBody,
	describeSourceError,
	EMPTY_DRAFT,
	type IssuedSource,
	type SourceDraft,
} from '#/domain/sources/model'
import { IssuedWebhook } from '#/domain/sources/ui/issued-webhook'
import { SourceFormView } from '#/domain/sources/ui/source-form-view'

export function SourceNewPage() {
	const navigate = useNavigate()
	const { agents, projects } = useSourceFormOptions()
	const create = useCreateSource()
	const [error, setError] = useState<string | null>(null)
	const [issued, setIssued] = useState<IssuedSource | null>(null)

	const submit = (draft: SourceDraft) => {
		setError(null)
		create.mutate(
			{ body: createBody(draft) },
			{
				onSuccess: setIssued,
				onError: (failure) => setError(describeSourceError(failure)),
			},
		)
	}

	const openSource = () => {
		if (issued)
			void navigate({
				to: '/sources/$sourceId/{-$tab}',
				params: { sourceId: issued.source.id },
			})
	}

	return (
		<Page>
			<header className="mb-4">
				<h1 className="m-0 text-xl leading-[1.2] font-semibold tracking-[-0.01em]">
					Nouvelle source
				</h1>
				<span className="text-muted-foreground">
					Un webhook que ton outil appelle ; chaque événement devient une tâche
				</span>
			</header>
			{issued ? (
				<div className="grid max-w-[880px] gap-3">
					<p className="m-0">Source « {issued.source.name} » créée.</p>
					<IssuedWebhook issued={issued} />
					<div className="flex gap-2">
						<Button type="button" onClick={openSource}>
							Ouvrir la source
						</Button>
						<Button asChild variant="outline">
							<Link to="/sources">Retour aux sources</Link>
						</Button>
					</div>
				</div>
			) : (
				<SourceFormView
					mode="create"
					initial={EMPTY_DRAFT}
					agents={agents}
					projects={projects}
					pending={create.isPending}
					error={error}
					onSubmit={submit}
					onCancel={() => void navigate({ to: '/sources' })}
				/>
			)}
		</Page>
	)
}
