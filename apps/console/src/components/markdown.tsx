import { type ReactNode, useMemo } from 'react'
import { cn } from '#/lib/utils'

/**
 * Rendu Markdown sûr : le texte devient des éléments React, jamais du HTML brut.
 * Titres (avec ancres), paragraphes, listes imbriquées et listes de tâches, citations,
 * blocs de code, tableaux GFM, séparateurs ; en ligne : code, gras, italique, barré et
 * liens (http, https, mailto ou relatifs seulement). Les niveaux de sévérité d'un audit
 * (Critique, Haute, Moyenne, Basse, Info) deviennent des pastilles.
 */

export type Variant = 'compact' | 'document'
export type Align = 'left' | 'center' | 'right' | null
export type HeadingLevel = 1 | 2 | 3 | 4 | 5 | 6

export interface ListItem {
	/** `null` hors liste de tâches. */
	checked: boolean | null
	blocks: Block[]
}

export type Block =
	| { type: 'heading'; level: HeadingLevel; text: string; id: string }
	| { type: 'paragraph'; text: string }
	| { type: 'code'; text: string; lang?: string }
	| { type: 'quote'; blocks: Block[] }
	| { type: 'list'; ordered: boolean; start: number; items: ListItem[] }
	| { type: 'table'; align: Align[]; header: string[]; rows: string[][] }
	| { type: 'rule' }

export interface TocEntry {
	id: string
	level: HeadingLevel
	text: string
}

const HEADING = /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/
const FENCE = /^(\s*)(`{3,}|~{3,})\s*([^\s`]*)/
const ITEM = /^(\s*)(?:([-*+])|(\d{1,9})[.)])([ \t]+)(.*)$/
const QUOTE = /^\s*>\s?(.*)$/
const RULE = /^\s*([-*_])(\s*\1){2,}\s*$/
const TASK = /^\[([ xX])\]\s+(.*)$/
const TABLE_SEPARATOR = /^\s*\|?\s*:?-+:?\s*(?:\|\s*:?-+:?\s*)*\|?\s*$/
const LANG = /^[\w#+.-]{1,32}$/

/** Colonnes d'indentation d'une ligne ; une tabulation avance au multiple de 4 suivant. */
function indentOf(line: string): number {
	let column = 0
	for (const char of line) {
		if (char === ' ') column += 1
		else if (char === '\t') column += 4 - (column % 4)
		else break
	}
	return column
}

function dedent(line: string, columns: number): string {
	let column = 0
	let index = 0
	while (index < line.length && column < columns) {
		const char = line[index]
		if (char === ' ') column += 1
		else if (char === '\t') {
			const width = 4 - (column % 4)
			if (column + width > columns) {
				return ' '.repeat(column + width - columns) + line.slice(index + 1)
			}
			column += width
		} else break
		index += 1
	}
	return line.slice(index)
}

interface ItemLine {
	indent: number
	ordered: boolean
	start: number
	contentColumn: number
	content: string
}

function itemOf(line: string): ItemLine | null {
	if (RULE.test(line)) return null
	const match = ITEM.exec(line)
	if (!match) return null
	const indent = indentOf(match[1] ?? '')
	const ordered = match[3] !== undefined
	const marker = ordered ? (match[3] ?? '').length + 1 : 1
	const gap = indentOf(
		`${match[1] ?? ''}${' '.repeat(marker)}${match[4] ?? ''}`,
	)
	const contentColumn = gap - indent - marker > 4 ? indent + marker + 1 : gap
	return {
		indent,
		ordered,
		start: ordered ? Number(match[3]) : 1,
		contentColumn,
		content: match[5] ?? '',
	}
}

/** Cellules d'une ligne de tableau ; `\|` est une barre littérale. */
export function splitRow(line: string): string[] {
	let row = line.trim()
	if (row.startsWith('|')) row = row.slice(1)
	if (row.endsWith('|') && !row.endsWith('\\|')) row = row.slice(0, -1)
	const cells: string[] = []
	let cell = ''
	for (let index = 0; index < row.length; index += 1) {
		const char = row[index]
		if (char === '\\' && row[index + 1] === '|') {
			cell += '|'
			index += 1
		} else if (char === '|') {
			cells.push(cell.trim())
			cell = ''
		} else cell += char
	}
	cells.push(cell.trim())
	return cells
}

function tableAt(lines: ReadonlyArray<string>, index: number): boolean {
	const header = lines[index] ?? ''
	const separator = lines[index + 1] ?? ''
	if (!header.includes('|') || !separator.includes('|')) return false
	if (!TABLE_SEPARATOR.test(separator)) return false
	return splitRow(header).length === splitRow(separator).length
}

function alignOf(cell: string): Align {
	const left = cell.startsWith(':')
	const right = cell.endsWith(':')
	if (left && right) return 'center'
	if (right) return 'right'
	if (left) return 'left'
	return null
}

function startsBlock(lines: ReadonlyArray<string>, index: number): boolean {
	const line = lines[index] ?? ''
	return (
		HEADING.test(line) ||
		FENCE.test(line) ||
		itemOf(line) !== null ||
		QUOTE.test(line) ||
		RULE.test(line) ||
		tableAt(lines, index)
	)
}

function nextNonBlank(lines: ReadonlyArray<string>, from: number): number {
	for (let index = from; index < lines.length; index += 1) {
		if ((lines[index] ?? '').trim()) return index
	}
	return -1
}

/** Texte brut d'un fragment Markdown en ligne (sommaire, ancres). */
export function plainText(text: string): string {
	return text
		.replace(/\[([^\]]+)\]\([^)\s]+\)/g, '$1')
		.replace(/\\([\\`*_{}[\]()#+\-.!|~<>])/g, '$1')
		.replace(/(\*\*|__|~~|[`*_])/g, '')
		.trim()
}

export function slugify(text: string): string {
	return text
		.normalize('NFD')
		.replace(/[̀-ͯ]/g, '')
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 80)
		.replace(/-+$/, '')
}

type Slugger = (text: string) => string

function createSlugger(): Slugger {
	const used = new Set<string>()
	return (text) => {
		const base = slugify(plainText(text)) || 'section'
		let id = base
		for (let suffix = 1; used.has(id); suffix += 1) id = `${base}-${suffix}`
		used.add(id)
		return id
	}
}

interface Parsed<T> {
	block: T
	next: number
}

function parseFence(
	lines: ReadonlyArray<string>,
	index: number,
): Parsed<Block> {
	const fence = FENCE.exec(lines[index] ?? '')
	const indent = indentOf(fence?.[1] ?? '')
	const marker = fence?.[2] ?? '```'
	const info = fence?.[3] ?? ''
	const body: string[] = []
	let cursor = index + 1
	while (cursor < lines.length) {
		const line = lines[cursor] ?? ''
		const trimmed = line.trim()
		if (
			trimmed.length >= marker.length &&
			[...trimmed].every((char) => char === marker[0])
		) {
			cursor += 1
			break
		}
		body.push(dedent(line, indent))
		cursor += 1
	}
	return {
		block: {
			type: 'code',
			text: body.join('\n'),
			...(LANG.test(info) ? { lang: info } : {}),
		},
		next: cursor,
	}
}

function parseTable(
	lines: ReadonlyArray<string>,
	index: number,
): Parsed<Block> {
	const header = splitRow(lines[index] ?? '')
	const align = splitRow(lines[index + 1] ?? '').map(alignOf)
	const rows: string[][] = []
	let cursor = index + 2
	while (cursor < lines.length) {
		const line = lines[cursor] ?? ''
		if (
			!line.trim() ||
			!line.includes('|') ||
			HEADING.test(line) ||
			FENCE.test(line) ||
			QUOTE.test(line)
		) {
			break
		}
		const cells = splitRow(line).slice(0, header.length)
		while (cells.length < header.length) cells.push('')
		rows.push(cells)
		cursor += 1
	}
	return { block: { type: 'table', align, header, rows }, next: cursor }
}

function parseList(
	lines: ReadonlyArray<string>,
	index: number,
	slug: Slugger,
): Parsed<Block> {
	const first = itemOf(lines[index] ?? '') as ItemLine
	const nested = first.indent + 2
	const items: ListItem[] = []
	let cursor = index
	while (cursor < lines.length) {
		const item = itemOf(lines[cursor] ?? '')
		if (!item || item.ordered !== first.ordered || item.indent >= nested) break
		const body = [item.content]
		let shift: number | null = null
		let afterBlank = false
		cursor += 1
		while (cursor < lines.length) {
			const line = lines[cursor] ?? ''
			if (!line.trim()) {
				const ahead = nextNonBlank(lines, cursor)
				if (ahead === -1) break
				const next = lines[ahead] ?? ''
				if (indentOf(next) >= nested) {
					for (let blank = cursor; blank < ahead; blank += 1) body.push('')
					cursor = ahead
					afterBlank = true
					continue
				}
				const following = itemOf(next)
				if (
					following &&
					following.ordered === first.ordered &&
					following.indent < nested
				) {
					cursor = ahead
				}
				break
			}
			const indent = indentOf(line)
			if (indent >= nested) {
				shift ??= Math.min(indent, item.contentColumn)
				body.push(dedent(line, shift))
				cursor += 1
				afterBlank = false
				continue
			}
			if (!afterBlank && !startsBlock(lines, cursor)) {
				body.push(line.trim())
				cursor += 1
				continue
			}
			break
		}
		const task = TASK.exec(body[0] ?? '')
		if (task) body[0] = task[2] ?? ''
		items.push({
			checked: task ? task[1] !== ' ' : null,
			blocks: parseLines(body, slug),
		})
	}
	return {
		block: {
			type: 'list',
			ordered: first.ordered,
			start: first.start,
			items,
		},
		next: cursor,
	}
}

function parseLines(lines: ReadonlyArray<string>, slug: Slugger): Block[] {
	const blocks: Block[] = []
	let index = 0
	while (index < lines.length) {
		const line = lines[index] ?? ''
		if (!line.trim()) {
			index += 1
			continue
		}
		if (FENCE.test(line)) {
			const { block, next } = parseFence(lines, index)
			blocks.push(block)
			index = next
			continue
		}
		const heading = HEADING.exec(line)
		if (heading) {
			const level = (heading[1]?.length ?? 1) as HeadingLevel
			const text = heading[2] ?? ''
			blocks.push({ type: 'heading', level, text, id: slug(text) })
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
			blocks.push({ type: 'quote', blocks: parseLines(body, slug) })
			continue
		}
		if (tableAt(lines, index)) {
			const { block, next } = parseTable(lines, index)
			blocks.push(block)
			index = next
			continue
		}
		if (itemOf(line)) {
			const { block, next } = parseList(lines, index, slug)
			blocks.push(block)
			index = next
			continue
		}
		const body: string[] = []
		while (
			index < lines.length &&
			(lines[index] ?? '').trim() &&
			(body.length === 0 || !startsBlock(lines, index))
		) {
			body.push((lines[index] ?? '').trim())
			index += 1
		}
		blocks.push({ type: 'paragraph', text: body.join('\n') })
	}
	return blocks
}

export function parseBlocks(source: string): Block[] {
	return parseLines(source.replace(/\r\n?/g, '\n').split('\n'), createSlugger())
}

/** Titres de premier niveau du document, jusqu'à `maxLevel`, pour un sommaire. */
export function tableOfContents(
	blocks: ReadonlyArray<Block>,
	maxLevel: HeadingLevel = 3,
): TocEntry[] {
	return blocks.flatMap((block) =>
		block.type === 'heading' && block.level <= maxLevel
			? [{ id: block.id, level: block.level, text: plainText(block.text) }]
			: [],
	)
}

/** `null` si l'URL n'est pas d'un schéma autorisé (javascript:, data:…). */
export function safeHref(url: string): string | null {
	const trimmed = url.trim()
	if (/^(https?:|mailto:)/i.test(trimmed)) return trimmed
	if (/^[a-z][a-z0-9+.-]*:/i.test(trimmed)) return null
	return trimmed.startsWith('//') ? null : trimmed
}

export type Severity = 'critical' | 'high' | 'medium' | 'low' | 'info'

const SEVERITY_WORDS: ReadonlyMap<string, Severity> = new Map([
	['critique', 'critical'],
	['critical', 'critical'],
	['haute', 'high'],
	['high', 'high'],
	['moyenne', 'medium'],
	['medium', 'medium'],
	['basse', 'low'],
	['low', 'low'],
	['info', 'info'],
	['informational', 'info'],
])

/** Niveau de sévérité si `text` est exactement l'un des mots reconnus. */
export function severityOf(text: string): Severity | null {
	return SEVERITY_WORDS.get(text.trim().toLowerCase()) ?? null
}

const SEVERITY_TONE: Readonly<Record<Severity, string>> = {
	critical: 'border-signal-danger/45 bg-signal-danger-soft text-signal-danger',
	high: 'border-signal-high/45 bg-signal-high-soft text-signal-high',
	medium:
		'border-signal-attention/50 bg-signal-attention-soft text-signal-attention-ink',
	low: 'border-signal-running/35 bg-signal-running-soft text-signal-running',
	info: 'border-border bg-signal-neutral-soft text-signal-neutral',
}

export function SeverityBadge({
	severity,
	children,
}: {
	severity: Severity
	children: ReactNode
}) {
	return (
		<span
			data-severity={severity}
			className={cn(
				'inline-flex items-center gap-1 rounded-[6px] border px-1.5 py-px align-[0.08em] text-[11.5px] leading-[1.35] font-medium whitespace-nowrap',
				SEVERITY_TONE[severity],
			)}
		>
			<span aria-hidden="true" className="size-1.5 rounded-full bg-current" />
			<span className="sr-only">Sévérité : </span>
			{children}
		</span>
	)
}

const INLINE =
	/(\\[\\`*_{}[\]()#+\-.!|~<>])|(`[^`]+`)|(\*\*[^*]+\*\*|__[^_]+__)|(\*[^*\s][^*]*\*|_[^_\s][^_]*_)|(~~[^~]+~~)|(\[[^\]]+\]\([^)\s]+\))|(https?:\/\/[^\s<>()]*[^\s<>().,;:!?»"'])/

const LINK_CLASS = 'text-primary underline-offset-2 hover:underline'

const INLINE_CODE: Readonly<Record<Variant, string>> = {
	compact:
		'rounded-sm bg-current/10 px-1 font-mono text-[0.9em] break-words [box-decoration-break:clone]',
	document:
		'rounded-[4px] border border-border/70 bg-secondary px-[0.3em] py-px font-mono text-[0.86em] break-words [box-decoration-break:clone]',
}

export function renderInline(
	text: string,
	keyPrefix = 'i',
	variant: Variant = 'compact',
): ReactNode[] {
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
			nodes.push(token.slice(1))
		} else if (match[2]) {
			nodes.push(
				<code key={id} className={INLINE_CODE[variant]}>
					{token.slice(1, -1)}
				</code>,
			)
		} else if (match[3]) {
			const inner = token.slice(2, -2)
			const severity = severityOf(inner)
			nodes.push(
				severity ? (
					<SeverityBadge key={id} severity={severity}>
						{inner.trim()}
					</SeverityBadge>
				) : (
					<strong key={id} className="font-semibold">
						{renderInline(inner, id, variant)}
					</strong>
				),
			)
		} else if (match[4]) {
			nodes.push(
				<em key={id}>{renderInline(token.slice(1, -1), id, variant)}</em>,
			)
		} else if (match[5]) {
			nodes.push(
				<del key={id}>{renderInline(token.slice(2, -2), id, variant)}</del>,
			)
		} else if (match[7]) {
			nodes.push(
				<a
					key={id}
					href={token}
					target="_blank"
					rel="noopener noreferrer"
					className={cn(LINK_CLASS, 'break-words')}
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
						{renderInline(label, id, variant)}
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

interface Styles {
	root: string
	heading: Readonly<Record<HeadingLevel, string>>
	prose: string
	code: string
	caption: string
	quote: string
	table: string
	th: string
	td: string
	row: string
	rule: string
	item: string
}

const STYLES: Readonly<Record<Variant, Styles>> = {
	compact: {
		root: 'flex flex-col gap-2.5 text-[13px] leading-[1.55]',
		heading: {
			1: 'text-lg font-semibold',
			2: 'text-base font-semibold',
			3: 'text-[14px] font-semibold',
			4: 'text-[13px] font-semibold',
			5: 'text-[13px] font-semibold',
			6: 'text-[13px] font-semibold opacity-75',
		},
		prose: '',
		code: 'overflow-hidden rounded-md bg-current/8',
		caption: 'px-3 pt-2 font-mono text-[11px] opacity-70',
		quote: 'border-l-2 border-current/30 pl-3 opacity-85',
		table: 'text-[12.5px]',
		th: 'border-b border-current/20 bg-current/8 px-2 py-1 font-semibold',
		td: 'border-t border-current/12 px-2 py-1',
		row: '',
		rule: 'border-current/20',
		item: 'gap-1',
	},
	document: {
		root: 'flex flex-col gap-3.5 text-[14.5px] leading-[1.65] text-foreground',
		heading: {
			1: 'mt-3 text-[23px] leading-[1.25] font-semibold tracking-[-0.018em] first:mt-0',
			2: 'mt-6 border-t pt-6 text-[18px] leading-[1.3] font-semibold tracking-[-0.012em] first:mt-0 first:border-t-0 first:pt-0',
			3: 'mt-3 text-[15.5px] leading-[1.35] font-semibold first:mt-0',
			4: 'mt-2 text-[14.5px] leading-[1.4] font-semibold first:mt-0',
			5: 'mt-2 text-[13px] leading-[1.4] font-semibold first:mt-0',
			6: 'mt-2 text-[13px] leading-[1.4] font-semibold text-muted-foreground first:mt-0',
		},
		prose: 'max-w-[75ch]',
		code: 'overflow-hidden rounded-lg border bg-secondary/55',
		caption:
			'border-b bg-secondary/70 px-3 py-1 font-mono text-[11px] tracking-[0.02em] text-muted-foreground',
		quote:
			'max-w-[75ch] border-l-[3px] border-border pl-4 text-muted-foreground',
		table: 'text-[13.5px] leading-[1.5]',
		th: 'border-b bg-secondary px-3 py-2 text-[12px] font-semibold tracking-[0.01em] text-muted-foreground',
		td: 'border-t px-3 py-2',
		row: 'even:bg-secondary/35',
		rule: 'my-3 border-border',
		item: 'gap-1.5',
	},
}

const ALIGN_CLASS: Readonly<Record<NonNullable<Align> | 'none', string>> = {
	left: 'text-left',
	center: 'text-center',
	right: 'text-right',
	none: 'text-left',
}

const BULLETS = ['list-disc', 'list-[circle]', 'list-[square]'] as const

interface RenderContext {
	variant: Variant
	styles: Styles
	depth: number
}

function renderLines(text: string, key: string, variant: Variant): ReactNode[] {
	const nodes: ReactNode[] = []
	let offset = 0
	for (const line of text.split('\n')) {
		if (offset > 0) nodes.push(<br key={`${key}-br-${offset}`} />)
		nodes.push(...renderInline(line, `${key}-${offset}`, variant))
		offset += line.length + 1
	}
	return nodes
}

function renderCell(text: string, key: string, variant: Variant): ReactNode {
	const bare = /^(\*\*|__)(.*)\1$/.exec(text)?.[2] ?? text
	const severity = severityOf(bare)
	return severity ? (
		<SeverityBadge severity={severity}>{bare.trim()}</SeverityBadge>
	) : (
		renderInline(text, key, variant)
	)
}

function renderTable(
	block: Extract<Block, { type: 'table' }>,
	key: string,
	{ variant, styles }: RenderContext,
): ReactNode {
	const alignOfColumn = (column: number) =>
		ALIGN_CLASS[block.align[column] ?? 'none']
	return (
		<div
			key={key}
			className={cn(
				'max-w-full overflow-x-auto rounded-md border',
				variant === 'compact' && 'border-current/20',
			)}
		>
			<table className={cn('figures w-full border-collapse', styles.table)}>
				<thead>
					<tr>
						{block.header.map((cell, column) => {
							const cellKey = `${key}-h${column}`
							return (
								<th
									key={cellKey}
									scope="col"
									className={cn(
										styles.th,
										'align-bottom',
										alignOfColumn(column),
									)}
								>
									{renderInline(cell, cellKey, variant)}
								</th>
							)
						})}
					</tr>
				</thead>
				<tbody>
					{block.rows.map((row, rowIndex) => {
						const rowKey = `${key}-r${rowIndex}`
						return (
							<tr key={rowKey} className={styles.row}>
								{row.map((cell, column) => {
									const cellKey = `${rowKey}-${column}`
									return (
										<td
											key={cellKey}
											className={cn(
												styles.td,
												'align-top',
												alignOfColumn(column),
											)}
										>
											{renderCell(cell, cellKey, variant)}
										</td>
									)
								})}
							</tr>
						)
					})}
				</tbody>
			</table>
		</div>
	)
}

function renderList(
	block: Extract<Block, { type: 'list' }>,
	key: string,
	context: RenderContext,
): ReactNode {
	const List = block.ordered ? 'ol' : 'ul'
	const inner = { ...context, depth: context.depth + 1 }
	return (
		<List
			key={key}
			start={block.ordered && block.start !== 1 ? block.start : undefined}
			className={cn(
				'm-0 pl-5',
				block.ordered
					? 'list-decimal'
					: BULLETS[Math.min(context.depth, BULLETS.length - 1)],
				context.depth === 0 && context.styles.prose,
				'marker:text-muted-foreground',
				context.variant === 'compact' && 'marker:text-current/70',
			)}
		>
			{block.items.map((item, itemIndex) => {
				const itemKey = `${key}-${itemIndex}`
				return (
					<li
						key={itemKey}
						className={cn(
							'pl-0.5',
							item.checked !== null && 'relative list-none',
						)}
					>
						{item.checked !== null ? (
							<input
								type="checkbox"
								checked={item.checked}
								disabled
								readOnly
								aria-label={item.checked ? 'Fait' : 'À faire'}
								className="absolute top-[0.32em] -left-5 size-3.5 accent-primary"
							/>
						) : null}
						<div className={cn('flex min-w-0 flex-col', context.styles.item)}>
							{renderBlocks(item.blocks, itemKey, inner)}
						</div>
					</li>
				)
			})}
		</List>
	)
}

function renderBlock(
	block: Block,
	key: string,
	context: RenderContext,
): ReactNode {
	const { variant, styles } = context
	switch (block.type) {
		case 'heading': {
			const Tag = `h${Math.min(block.level + 1, 6)}` as 'h2'
			return (
				<Tag
					key={key}
					id={
						variant === 'document' && context.depth === 0 ? block.id : undefined
					}
					className={cn('m-0 scroll-mt-4', styles.heading[block.level])}
				>
					{renderInline(block.text, key, variant)}
				</Tag>
			)
		}
		case 'paragraph':
			return (
				<p key={key} className={cn('m-0', styles.prose)}>
					{renderLines(block.text, key, variant)}
				</p>
			)
		case 'code':
			return (
				<figure key={key} className={cn('m-0 max-w-full', styles.code)}>
					{block.lang ? (
						<figcaption className={styles.caption}>{block.lang}</figcaption>
					) : null}
					<pre className="m-0 overflow-x-auto p-3 font-mono text-[12.5px] leading-[1.55] whitespace-pre [tab-size:2]">
						<code>{block.text}</code>
					</pre>
				</figure>
			)
		case 'quote':
			return (
				<blockquote
					key={key}
					className={cn('m-0 flex flex-col gap-2', styles.quote)}
				>
					{renderBlocks(block.blocks, key, { ...context, depth: 1 })}
				</blockquote>
			)
		case 'list':
			return renderList(block, key, context)
		case 'table':
			return renderTable(block, key, context)
		case 'rule':
			return <hr key={key} className={cn('m-0 w-full', styles.rule)} />
	}
}

function renderBlocks(
	blocks: ReadonlyArray<Block>,
	prefix: string,
	context: RenderContext,
): ReactNode[] {
	return blocks.map((block, index) =>
		renderBlock(block, `${prefix}-${index}`, context),
	)
}

export function Markdown({
	source,
	className,
	variant = 'compact',
}: {
	source: string
	className?: string
	variant?: Variant
}) {
	const blocks = useMemo(() => parseBlocks(source), [source])
	const styles = STYLES[variant]
	return (
		<div data-variant={variant} className={cn(styles.root, className)}>
			{renderBlocks(blocks, 'b', { variant, styles, depth: 0 })}
		</div>
	)
}
