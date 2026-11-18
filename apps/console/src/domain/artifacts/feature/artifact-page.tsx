import { Link } from '@tanstack/react-router'
import {
	Check,
	Copy,
	Download,
	ListTree,
	SquareArrowOutUpRight,
} from 'lucide-react'
import { type MouseEvent, useEffect, useMemo, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import {
	parseBlocks,
	type TocEntry,
	tableOfContents,
} from '#/components/markdown'
import { Button } from '#/components/ui/button'
import { useArtifact } from '#/domain/artifacts/hooks/use-artifacts'
import {
	type ArtifactDetail,
	artifactFileName,
	kindLabel,
} from '#/domain/artifacts/model'
import { ArtifactContent } from '#/domain/artifacts/ui/artifact-content'
import { useAgentHue } from '#/domain/audit/hooks/use-agent-hue'
import { relativeAge } from '#/domain/dashboard/format'
import { AgentChip } from '#/domain/tasks/ui/agent-chip'
import { Sev } from '#/domain/tasks/ui/sev'
import { useCopyToClipboard } from '#/hooks/use-copy-to-clipboard'
import { shortName } from '#/lib/agent-name'
import { cn } from '#/lib/utils'

const DATE = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'long',
	timeStyle: 'short',
})

function downloadText(content: string, filename: string, type: string) {
	const url = URL.createObjectURL(new Blob([content], { type }))
	const anchor = document.createElement('a')
	anchor.href = url
	anchor.download = filename
	document.body.append(anchor)
	anchor.click()
	anchor.remove()
	setTimeout(() => URL.revokeObjectURL(url), 0)
}

/** Titre du sommaire actuellement en haut de la zone de lecture. */
function useActiveHeading(entries: ReadonlyArray<TocEntry>) {
	const [active, setActive] = useState<string | null>(null)
	useEffect(() => {
		if (typeof IntersectionObserver === 'undefined' || entries.length === 0) {
			return
		}
		const observer = new IntersectionObserver(
			(observed) => {
				const top = observed
					.filter((entry) => entry.isIntersecting)
					.sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)
				if (top[0]) setActive(top[0].target.id)
			},
			{ rootMargin: '0px 0px -70% 0px' },
		)
		for (const entry of entries) {
			const element = document.getElementById(entry.id)
			if (element) observer.observe(element)
		}
		return () => observer.disconnect()
	}, [entries])
	return [active, setActive] as const
}

const TOC_INDENT: Readonly<Record<number, string>> = {
	1: 'pl-2.5 font-medium',
	2: 'pl-2.5',
	3: 'pl-5.5 text-[12px]',
}

function TocLinks({
	entries,
	active,
	onSelect,
}: {
	entries: ReadonlyArray<TocEntry>
	active: string | null
	onSelect: (id: string) => void
}) {
	const hasH1 = entries.some((entry) => entry.level === 1)
	function go(event: MouseEvent<HTMLAnchorElement>, id: string) {
		event.preventDefault()
		const target = document.getElementById(id)
		if (!target) return
		const reduce = window.matchMedia?.(
			'(prefers-reduced-motion: reduce)',
		).matches
		target.scrollIntoView({
			behavior: reduce ? 'auto' : 'smooth',
			block: 'start',
		})
		onSelect(id)
	}
	return (
		<ol className="m-0 flex list-none flex-col p-0 text-[12.5px] leading-[1.4]">
			{entries.map((entry) => {
				const level = hasH1 ? entry.level : Math.max(1, entry.level - 1)
				const current = entry.id === active
				return (
					<li key={entry.id}>
						<a
							href={`#${entry.id}`}
							onClick={(event) => go(event, entry.id)}
							aria-current={current ? 'location' : undefined}
							className={cn(
								'-ml-px block border-l-2 border-transparent py-[5px] pr-2 text-muted-foreground transition-colors duration-150 hover:text-foreground',
								TOC_INDENT[level],
								current && 'border-primary text-foreground',
							)}
						>
							{entry.text}
						</a>
					</li>
				)
			})}
		</ol>
	)
}

function Actions({ artifact }: { artifact: ArtifactDetail }) {
	const { copied, copy } = useCopyToClipboard()
	const content = artifact.content ?? ''
	const json = artifact.kind === 'json'
	return (
		<>
			{content ? (
				<>
					<Button type="button" variant="outline" onClick={() => copy(content)}>
						{copied ? (
							<Check aria-hidden="true" />
						) : (
							<Copy aria-hidden="true" />
						)}
						{copied ? 'Copié' : json ? 'Copier le JSON' : 'Copier le Markdown'}
					</Button>
					<Button
						type="button"
						variant="outline"
						onClick={() =>
							downloadText(
								content,
								artifactFileName(artifact.title, artifact.kind),
								json
									? 'application/json;charset=utf-8'
									: 'text/markdown;charset=utf-8',
							)
						}
					>
						<Download aria-hidden="true" />
						{json ? 'Télécharger (.json)' : 'Télécharger (.md)'}
					</Button>
				</>
			) : null}
			{artifact.card_id ? (
				<Button asChild variant="outline">
					<Link to="/tasks/$taskId" params={{ taskId: artifact.card_id }}>
						<SquareArrowOutUpRight aria-hidden="true" />
						Ouvrir la tâche
					</Link>
				</Button>
			) : null}
		</>
	)
}

function ArtifactView({ artifact }: { artifact: ArtifactDetail }) {
	const hueOf = useAgentHue()
	const content = artifact.content ?? ''
	const toc = useMemo(
		() =>
			artifact.kind === 'markdown' && content
				? tableOfContents(parseBlocks(content))
				: [],
		[artifact.kind, content],
	)
	const [active, setActive] = useActiveHeading(toc)
	const showToc = toc.length >= 2
	const updated = new Date(artifact.updated_at)

	return (
		<Page>
			<PageHeader
				title={artifact.title}
				meta={<Sev tone="info">{kindLabel(artifact.kind)}</Sev>}
				actions={<Actions artifact={artifact} />}
			/>
			<div className="-mt-1 mb-4 flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1.5 text-xs text-muted-foreground">
				{artifact.agent_name ? (
					<AgentChip
						name={shortName(artifact.agent_name)}
						hue={hueOf(artifact.agent_id, artifact.agent_name)}
						size={16}
					/>
				) : (
					<span>{artifact.created_by}</span>
				)}
				{artifact.card_id ? (
					<>
						<span aria-hidden="true" className="text-faint">
							·
						</span>
						<Link
							to="/tasks/$taskId"
							params={{ taskId: artifact.card_id }}
							className="min-w-0 truncate text-primary hover:underline"
						>
							{artifact.card_title ?? 'Voir la tâche'}
						</Link>
					</>
				) : null}
				<span aria-hidden="true" className="text-faint">
					·
				</span>
				<time
					dateTime={artifact.updated_at}
					title={
						Number.isNaN(updated.getTime()) ? undefined : DATE.format(updated)
					}
				>
					Mis à jour {relativeAge(artifact.updated_at)}
				</time>
			</div>
			<div
				className={cn(
					'grid max-w-[1240px] gap-4',
					showToc && 'lg:grid-cols-[minmax(0,1fr)_216px] lg:gap-8',
				)}
			>
				{showToc ? (
					<details className="group rounded-lg border bg-card lg:hidden">
						<summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2 text-[12px] font-semibold [&::-webkit-details-marker]:hidden">
							<ListTree
								aria-hidden="true"
								className="size-4 text-muted-foreground"
							/>
							Sommaire
							<span className="ml-auto text-[11.5px] font-normal text-faint">
								{toc.length} sections
							</span>
						</summary>
						<nav
							aria-label="Sommaire du document"
							className="border-t px-3 py-2"
						>
							<TocLinks entries={toc} active={active} onSelect={setActive} />
						</nav>
					</details>
				) : null}
				<article className="min-w-0 rounded-lg border bg-card px-5 py-6 sm:px-8 sm:py-7 lg:px-10 lg:py-9">
					<ArtifactContent artifact={artifact} />
				</article>
				{showToc ? (
					<aside className="hidden lg:block">
						<nav
							aria-label="Sommaire"
							className="sticky top-4 max-h-[calc(100dvh-2rem)] overflow-y-auto pb-4"
						>
							<h2 className="m-0 mb-2 px-2.5 text-[11.5px] font-semibold tracking-[0.04em] text-muted-foreground uppercase">
								Sommaire
							</h2>
							<div className="border-l">
								<TocLinks entries={toc} active={active} onSelect={setActive} />
							</div>
						</nav>
					</aside>
				) : null}
			</div>
		</Page>
	)
}

export function ArtifactPage({ artifactId }: { artifactId: string }) {
	const query = useArtifact(artifactId)
	if (query.isPending) {
		return (
			<Page>
				<output className="block text-[13px] text-muted-foreground">
					Chargement…
				</output>
			</Page>
		)
	}
	if (query.isError) {
		return (
			<Page>
				<InlineAlert tone="danger">
					Impossible de charger cet artifact. Il a peut-être été supprimé.
				</InlineAlert>
			</Page>
		)
	}
	return <ArtifactView artifact={query.data} />
}
