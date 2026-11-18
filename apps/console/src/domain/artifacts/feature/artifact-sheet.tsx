import { Link } from '@tanstack/react-router'
import { ExternalLink } from 'lucide-react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Markdown } from '#/components/markdown'
import {
	Sheet,
	SheetBody,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
} from '#/components/ui/sheet'
import { useArtifact } from '#/domain/artifacts/hooks/use-artifacts'
import {
	type Artifact,
	kindLabel,
	prettyJson,
	safeUrl,
} from '#/domain/artifacts/model'
import { relativeAge } from '#/domain/dashboard/format'

interface Props {
	artifact: Artifact | null
	onClose: () => void
}

function OpenLink({ url }: { url: string | null | undefined }) {
	const href = safeUrl(url)
	if (!href) return null
	return (
		<a
			href={href}
			target="_blank"
			rel="noopener noreferrer"
			className="inline-flex items-center gap-1.5 text-[13px] text-primary hover:underline"
		>
			<ExternalLink aria-hidden="true" className="size-4" />
			Ouvrir le lien
		</a>
	)
}

function Content({ artifact }: { artifact: Artifact }) {
	const detail = useArtifact(artifact.id)
	if (detail.isPending) {
		return (
			<output className="block text-[13px] text-muted-foreground">
				Chargement…
			</output>
		)
	}
	if (detail.isError) {
		return (
			<InlineAlert tone="danger">
				Impossible de charger le contenu de l'artifact.
			</InlineAlert>
		)
	}
	const content = detail.data.content ?? ''
	switch (detail.data.kind) {
		case 'markdown':
			return content ? (
				<Markdown source={content} />
			) : (
				<p className="m-0 text-muted-foreground">Document vide.</p>
			)
		case 'json':
			return (
				<pre className="m-0 overflow-x-auto rounded-md bg-secondary p-3 font-mono text-xs">
					<code>{prettyJson(content)}</code>
				</pre>
			)
		case 'link':
			return (
				<div className="flex flex-col gap-2">
					<OpenLink url={detail.data.url} />
					{content ? <Markdown source={content} /> : null}
				</div>
			)
	}
}

export function ArtifactSheet({ artifact, onClose }: Props) {
	return (
		<Sheet open={artifact !== null} onOpenChange={(open) => !open && onClose()}>
			<SheetContent className="sm:max-w-[640px]">
				{artifact ? (
					<>
						<SheetHeader>
							<SheetTitle>{artifact.title}</SheetTitle>
							<SheetDescription>
								{kindLabel(artifact.kind)}
								{artifact.agent_name ? ` · ${artifact.agent_name}` : ''} · mis à
								jour {relativeAge(artifact.updated_at)}
							</SheetDescription>
							{artifact.card_id ? (
								<Link
									to="/tasks/$taskId"
									params={{ taskId: artifact.card_id }}
									className="truncate text-xs text-primary hover:underline"
								>
									{artifact.card_title ?? 'Voir la tâche'}
								</Link>
							) : null}
						</SheetHeader>
						<SheetBody>
							<Content artifact={artifact} />
						</SheetBody>
					</>
				) : null}
			</SheetContent>
		</Sheet>
	)
}
