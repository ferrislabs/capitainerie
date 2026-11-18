import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

/**
 * Rendu Markdown minimal et sûr : le texte devient des éléments React, jamais du HTML brut.
 * Titres, paragraphes, listes, citations, blocs de code, séparateurs ; en ligne : code,
 * gras, italique et liens (http, https, mailto ou relatifs seulement).
 */

export type Block =
	| { type: 'heading'; level: 1 | 2 | 3 | 4 | 5 | 6; text: string }
	| { type: 'paragraph'; text: string }
	| { type: 'code'; text: string }
	| { type: 'quote'; text: string }
	| { type: 'list'; ordered: boolean; items: string[] }
	| { type: 'rule' }

const HEADING = /^(#{1,6})\s+(.*?)\s*#*\s*$/
const FENCE = /^\s*(```|~~~)/
const BULLET = /^\s*[-*+]\s+(.*)$/
const ORDERED = /^\s*\d+[.)]\s+(.*)$/
const QUOTE = /^\s*>\s?(.*)$/
const RULE = /^\s*([-*_])(\s*\1){2,}\s*$/

function startsBlock(line: string): boolean {
	return (
		HEADING.test(line) ||
		FENCE.test(line) ||
		BULLET.test(line) ||
		ORDERED.test(line) ||
		QUOTE.test(line) ||
		RULE.test(line)
	)
}

export function parseBlocks(source: string): Block[] {
	const lines = source.replace(/\r\n?/g, '\n').split('\n')
	const blocks: Block[] = []
	let index = 0
	while (index < lines.length) {
		const line = lines[index] ?? ''
		if (!line.trim()) {
			index += 1
			continue
		}
		const fence = FENCE.exec(line)
		if (fence) {
			const marker = fence[1] ?? '```'
			const body: string[] = []
			index += 1
			while (
				index < lines.length &&
				!(lines[index] ?? '').trim().startsWith(marker)
			) {
				body.push(lines[index] ?? '')
				index += 1
			}
			index += 1
			blocks.push({ type: 'code', text: body.join('\n') })
			continue
		}
		const heading = HEADING.exec(line)
		if (heading) {
			const level = (heading[1]?.length ?? 1) as 1 | 2 | 3 | 4 | 5 | 6
			blocks.push({ type: 'heading', level, text: heading[2] ?? '' })
			index += 1
			continue
		}
		if (RULE.test(line)) {
			blocks.push({ type: 'rule' })
			index += 1
			continue
		}
		if (QUOTE.test(line)) {
			const body: string[] = []
			while (index < lines.length && QUOTE.test(lines[index] ?? '')) {
				body.push(QUOTE.exec(lines[index] ?? '')?.[1] ?? '')
				index += 1
			}
			blocks.push({ type: 'quote', text: body.join('\n') })
			continue
		}
		const ordered = ORDERED.test(line)
		if (ordered || BULLET.test(line)) {
			const pattern = ordered ? ORDERED : BULLET
			const items: string[] = []
			while (index < lines.length && pattern.test(lines[index] ?? '')) {
				items.push(pattern.exec(lines[index] ?? '')?.[1] ?? '')
				index += 1
			}
			blocks.push({ type: 'list', ordered, items })
			continue
		}
		const body: string[] = []
		while (
			index < lines.length &&
			(lines[index] ?? '').trim() &&
			!startsBlock(lines[index] ?? '')
		) {
			body.push((lines[index] ?? '').trim())
			index += 1
		}
		blocks.push({ type: 'paragraph', text: body.join('\n') })
	}
	return blocks
}

/** `null` si l'URL n'est pas d'un schéma autorisé (javascript:, data:…). */
export function safeHref(url: string): string | null {
	const trimmed = url.trim()
	if (/^(https?:|mailto:)/i.test(trimmed)) return trimmed
	if (/^[a-z][a-z0-9+.-]*:/i.test(trimmed)) return null
	return trimmed.startsWith('//') ? null : trimmed
}

const INLINE =
	/(`[^`]+`)|(\*\*[^*]+\*\*|__[^_]+__)|(\*[^*\s][^*]*\*|_[^_\s][^_]*_)|(\[[^\]]+\]\([^)\s]+\))|(https?:\/\/[^\s<>()]*[^\s<>().,;:!?»"'])/

const LINK_CLASS = 'text-primary underline-offset-2 hover:underline'

export function renderInline(text: string, keyPrefix = 'i'): ReactNode[] {
	const nodes: ReactNode[] = []
	let rest = text
	let key = 0
	while (rest) {
		const match = INLINE.exec(rest)
		if (!match) {
			nodes.push(rest)
			break
		}
		if (match.index > 0) nodes.push(rest.slice(0, match.index))
		const token = match[0]
		const id = `${keyPrefix}-${key++}`
		if (match[1]) {
			nodes.push(
				<code
					key={id}
					className="rounded-sm bg-secondary px-1 font-mono text-[0.9em]"
				>
					{token.slice(1, -1)}
				</code>,
			)
		} else if (match[2]) {
			nodes.push(
				<strong key={id}>{renderInline(token.slice(2, -2), id)}</strong>,
			)
		} else if (match[3]) {
			nodes.push(<em key={id}>{renderInline(token.slice(1, -1), id)}</em>)
		} else if (match[5]) {
			nodes.push(
				<a
					key={id}
					href={token}
					target="_blank"
					rel="noopener noreferrer"
					className={LINK_CLASS}
				>
					{token}
				</a>,
			)
		} else {
			const link = /^\[([^\]]+)\]\(([^)\s]+)\)$/.exec(token)
			const label = link?.[1] ?? token
			const href = safeHref(link?.[2] ?? '')
			nodes.push(
				href ? (
					<a
						key={id}
						href={href}
						target="_blank"
						rel="noopener noreferrer"
						className={LINK_CLASS}
					>
						{renderInline(label, id)}
					</a>
				) : (
					<span key={id}>{label}</span>
				),
			)
		}
		rest = rest.slice(match.index + token.length)
	}
	return nodes
}

const HEADING_CLASS: Readonly<Record<number, string>> = {
	1: 'text-lg font-semibold',
	2: 'text-base font-semibold',
	3: 'text-[14px] font-semibold',
	4: 'text-[13px] font-semibold',
	5: 'text-[13px] font-semibold',
	6: 'text-[13px] font-semibold text-muted-foreground',
}

function renderLines(text: string, key: string): ReactNode[] {
	const nodes: ReactNode[] = []
	let offset = 0
	for (const line of text.split('\n')) {
		if (offset > 0) nodes.push(<br key={`${key}-br-${offset}`} />)
		nodes.push(...renderInline(line, `${key}-${offset}`))
		offset += line.length + 1
	}
	return nodes
}

function renderBlock(block: Block, index: number): ReactNode {
	const key = `b${index}`
	switch (block.type) {
		case 'heading': {
			const Tag = `h${Math.min(block.level + 1, 6)}` as 'h2'
			return (
				<Tag key={key} className={cn('m-0', HEADING_CLASS[block.level])}>
					{renderInline(block.text, key)}
				</Tag>
			)
		}
		case 'paragraph':
			return (
				<p key={key} className="m-0">
					{renderLines(block.text, key)}
				</p>
			)
		case 'code':
			return (
				<pre
					key={key}
					className="m-0 overflow-x-auto rounded-md bg-secondary p-3 font-mono text-xs"
				>
					<code>{block.text}</code>
				</pre>
			)
		case 'quote':
			return (
				<blockquote
					key={key}
					className="m-0 border-l-2 pl-3 text-muted-foreground"
				>
					{renderLines(block.text, key)}
				</blockquote>
			)
		case 'list': {
			const List = block.ordered ? 'ol' : 'ul'
			return (
				<List
					key={key}
					className={cn(
						'm-0 pl-5',
						block.ordered ? 'list-decimal' : 'list-disc',
					)}
				>
					{block.items.map((item, itemIndex) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: liste figée, rendue une seule fois.
						<li key={itemIndex}>{renderInline(item, `${key}-${itemIndex}`)}</li>
					))}
				</List>
			)
		}
		case 'rule':
			return <hr key={key} className="m-0 border-border" />
	}
}

export function Markdown({
	source,
	className,
}: {
	source: string
	className?: string
}) {
	return (
		<div
			className={cn(
				'flex flex-col gap-2.5 text-[13px] leading-[1.55]',
				className,
			)}
		>
			{parseBlocks(source).map(renderBlock)}
		</div>
	)
}
