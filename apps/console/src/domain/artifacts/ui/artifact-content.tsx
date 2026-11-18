import { ExternalLink } from 'lucide-react'
import { Markdown } from '#/components/markdown'
import {
	type ArtifactDetail,
	prettyJson,
	safeUrl,
} from '#/domain/artifacts/model'

export function OpenLink({ url }: { url: string | null | undefined }) {
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

/** Contenu d'un artifact selon son type : document lu, JSON indenté ou lien. */
export function ArtifactContent({ artifact }: { artifact: ArtifactDetail }) {
	const content = artifact.content ?? ''
	switch (artifact.kind) {
		case 'markdown':
			return content ? (
				<Markdown source={content} variant="document" />
			) : (
				<p className="m-0 text-muted-foreground">Document vide.</p>
			)
		case 'json':
			return (
				<pre className="m-0 overflow-x-auto rounded-md border bg-secondary/55 p-3 font-mono text-[12.5px] leading-[1.55]">
					<code>{prettyJson(content)}</code>
				</pre>
			)
		case 'link':
			return (
				<div className="flex flex-col gap-3">
					<OpenLink url={artifact.url} />
					{content ? <Markdown source={content} variant="document" /> : null}
				</div>
			)
	}
}
