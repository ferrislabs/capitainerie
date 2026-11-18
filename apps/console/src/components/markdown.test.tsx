import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import {
	type Block,
	Markdown,
	parseBlocks,
	safeHref,
	severityOf,
	slugify,
	splitRow,
	tableOfContents,
} from '#/components/markdown'

describe('parseBlocks', () => {
	it('splits headings, lists, quotes, code and paragraphs', () => {
		const blocks = parseBlocks(
			'# Titre\n\nUn paragraphe\nsur deux lignes\n\n- a\n- b\n\n1. un\n2. deux\n\n> cité\n\n```\nconst x = 1\n```\n\n---',
		)
		expect(blocks.map((b) => b.type)).toEqual([
			'heading',
			'paragraph',
			'list',
			'list',
			'quote',
			'code',
			'rule',
		])
		expect(blocks[1]).toEqual({
			type: 'paragraph',
			text: 'Un paragraphe\nsur deux lignes',
		})
		expect(blocks[5]).toEqual({ type: 'code', text: 'const x = 1' })
	})

	it('keeps the fence language', () => {
		expect(parseBlocks('```rust\nfn main() {}\n```')).toEqual([
			{ type: 'code', text: 'fn main() {}', lang: 'rust' },
		])
		expect(parseBlocks('~~~~ ts title="x"\na\n~~~~')[0]).toEqual({
			type: 'code',
			text: 'a',
			lang: 'ts',
		})
		expect(parseBlocks('```<x>\na\n```')[0]).toEqual({
			type: 'code',
			text: 'a',
		})
	})
})

describe('tables', () => {
	const table = (source: string) =>
		parseBlocks(source).find(
			(b): b is Extract<Block, { type: 'table' }> => b.type === 'table',
		)

	it('reads header, alignment and rows', () => {
		expect(
			table(
				'| Sévérité | Fichier | Lignes |\n|:---|:--:|--:|\n| Haute | `a.rs:4` | 12 |',
			),
		).toEqual({
			type: 'table',
			align: ['left', 'center', 'right'],
			header: ['Sévérité', 'Fichier', 'Lignes'],
			rows: [['Haute', '`a.rs:4`', '12']],
		})
	})

	it('accepts rows without outer pipes and keeps escaped pipes', () => {
		expect(table('a | b\n--- | ---\nx \\| y | z')).toMatchObject({
			align: [null, null],
			header: ['a', 'b'],
			rows: [['x | y', 'z']],
		})
		expect(splitRow('| a \\| b | c |')).toEqual(['a | b', 'c'])
	})

	it('pads short rows and drops extra cells', () => {
		expect(
			table('| a | b | c |\n|---|---|---|\n| 1 |\n| 1 | 2 | 3 | 4 |')?.rows,
		).toEqual([
			['1', '', ''],
			['1', '2', '3'],
		])
	})

	it('needs a matching separator row', () => {
		expect(table('| a | b |\n|---|\n| 1 | 2 |')).toBeUndefined()
		expect(table('a | b\nplain text')).toBeUndefined()
	})

	it('interrupts a paragraph and stops at a blank line', () => {
		const blocks = parseBlocks(
			'Résumé :\n| a | b |\n|---|---|\n| 1 | 2 |\n\nFin.',
		)
		expect(blocks.map((b) => b.type)).toEqual([
			'paragraph',
			'table',
			'paragraph',
		])
	})
})

describe('lists', () => {
	it('nests by indentation and mixes list kinds', () => {
		const [list] = parseBlocks(
			'- un\n  - un.a\n    1. profond\n- deux\n\tsuite tabulée',
		)
		expect(list).toEqual({
			type: 'list',
			ordered: false,
			start: 1,
			items: [
				{
					checked: null,
					blocks: [
						{ type: 'paragraph', text: 'un' },
						{
							type: 'list',
							ordered: false,
							start: 1,
							items: [
								{
									checked: null,
									blocks: [
										{ type: 'paragraph', text: 'un.a' },
										{
											type: 'list',
											ordered: true,
											start: 1,
											items: [
												{
													checked: null,
													blocks: [{ type: 'paragraph', text: 'profond' }],
												},
											],
										},
									],
								},
							],
						},
					],
				},
				{
					checked: null,
					blocks: [{ type: 'paragraph', text: 'deux\nsuite tabulée' }],
				},
			],
		})
	})

	it('keeps the start number and continuation lines', () => {
		const [list] = parseBlocks(
			'3. trois\nsuite paresseuse\n4. quatre\n\n5. cinq',
		)
		expect(list).toMatchObject({ ordered: true, start: 3 })
		expect(list?.type === 'list' && list.items.length).toBe(3)
		expect(list?.type === 'list' && list.items[0]?.blocks[0]).toEqual({
			type: 'paragraph',
			text: 'trois\nsuite paresseuse',
		})
	})

	it('splits ordered and unordered siblings into two lists', () => {
		expect(parseBlocks('- a\n1. b').map((b) => b.type)).toEqual([
			'list',
			'list',
		])
	})

	it('nests code blocks inside an item', () => {
		const [list] = parseBlocks('1. étape\n\n   ```sh\n   make\n   ```\n\nAprès')
		expect(list?.type === 'list' && list.items[0]?.blocks[1]).toEqual({
			type: 'code',
			text: 'make',
			lang: 'sh',
		})
		expect(
			parseBlocks('1. étape\n\n   ```sh\n   make\n   ```\n\nAprès')[1],
		).toEqual({
			type: 'paragraph',
			text: 'Après',
		})
	})

	it('reads task items', () => {
		const [list] = parseBlocks('- [x] fait\n- [ ] à faire\n- normal')
		expect(list?.type === 'list' && list.items.map((i) => i.checked)).toEqual([
			true,
			false,
			null,
		])
	})
})

describe('headings', () => {
	it('gets slugified, deduplicated ids and a table of contents', () => {
		const blocks = parseBlocks(
			'# Audit sécurité\n\n## Résumé\n\n## Résumé\n\n### `jwks.rs` : **clés**\n\n#### Détail\n\n## !!!',
		)
		expect(blocks.flatMap((b) => (b.type === 'heading' ? [b.id] : []))).toEqual(
			[
				'audit-securite',
				'resume',
				'resume-1',
				'jwks-rs-cles',
				'detail',
				'section',
			],
		)
		expect(tableOfContents(blocks)).toEqual([
			{ id: 'audit-securite', level: 1, text: 'Audit sécurité' },
			{ id: 'resume', level: 2, text: 'Résumé' },
			{ id: 'resume-1', level: 2, text: 'Résumé' },
			{ id: 'jwks-rs-cles', level: 3, text: 'jwks.rs : clés' },
			{ id: 'section', level: 2, text: '!!!' },
		])
		expect(slugify('Élévation de privilèges (CVE-2024)')).toBe(
			'elevation-de-privileges-cve-2024',
		)
	})
})

describe('severityOf', () => {
	it('matches the exact words only', () => {
		expect(severityOf('Critique')).toBe('critical')
		expect(severityOf(' HAUTE ')).toBe('high')
		expect(severityOf('medium')).toBe('medium')
		expect(severityOf('Basse')).toBe('low')
		expect(severityOf('Informational')).toBe('info')
		expect(severityOf('Haute priorité')).toBeNull()
		expect(severityOf('Critiques')).toBeNull()
		expect(severityOf('constructor')).toBeNull()
		expect(severityOf('')).toBeNull()
	})
})

describe('safeHref', () => {
	it('allows web, mail and relative links only', () => {
		expect(safeHref('https://a.test')).toBe('https://a.test')
		expect(safeHref('mailto:x@a.test')).toBe('mailto:x@a.test')
		expect(safeHref('/tasks')).toBe('/tasks')
		expect(safeHref('javascript:alert(1)')).toBeNull()
		expect(safeHref('JaVaScRiPt:alert(1)')).toBeNull()
		expect(safeHref('data:text/html,x')).toBeNull()
		expect(safeHref('//evil.test')).toBeNull()
	})
})

describe('Markdown', () => {
	it('renders formatting as elements, never raw HTML', () => {
		const { container } = render(
			<Markdown
				source={
					"## Rapport\n\nDu **gras**, de l'*italique*, du `code` et [un lien](https://a.test).\n\n<script>alert(1)</script> <img src=x onerror=alert(1)>"
				}
			/>,
		)
		expect(screen.getByRole('heading', { name: 'Rapport' })).toBeTruthy()
		expect(container.querySelector('strong')?.textContent).toBe('gras')
		expect(container.querySelector('em')?.textContent).toBe('italique')
		expect(container.querySelector('code')?.textContent).toBe('code')
		const link = screen.getByRole('link', { name: 'un lien' })
		expect(link.getAttribute('href')).toBe('https://a.test')
		expect(link.getAttribute('rel')).toBe('noopener noreferrer')
		expect(container.querySelector('script')).toBeNull()
		expect(container.querySelector('img')).toBeNull()
		expect(container.textContent).toContain('<script>alert(1)</script>')
	})

	it('links bare web addresses without their trailing punctuation', () => {
		render(
			<Markdown source="Claude Code a ouvert une pull request en brouillon : https://github.com/acme/web/pull/12." />,
		)
		const link = screen.getByRole('link', {
			name: 'https://github.com/acme/web/pull/12',
		})
		expect(link.getAttribute('href')).toBe(
			'https://github.com/acme/web/pull/12',
		)
		expect(link.getAttribute('rel')).toBe('noopener noreferrer')
	})

	it('renders tables, severity pills, nested and task lists', () => {
		const { container } = render(
			<Markdown
				variant="document"
				source={
					'## Constats\n\n| Sévérité | Constat |\n|---|---|\n| Critique | Clé **en clair** |\n| Haute priorité | **Info** |\n\n- [x] corrigé\n  - sous-point\n\n```rust\nlet a = 1;\n```'
				}
			/>,
		)
		const table = screen.getByRole('table')
		expect(
			screen.getAllByRole('columnheader').map((c) => c.textContent),
		).toEqual(['Sévérité', 'Constat'])
		expect(table.querySelector('[data-severity="critical"]')?.textContent).toBe(
			'Sévérité : Critique',
		)
		expect(table.querySelector('[data-severity="info"]')).toBeTruthy()
		expect(
			screen.getByText('Haute priorité').closest('[data-severity]'),
		).toBeNull()
		expect(container.querySelectorAll('[data-severity]')).toHaveLength(2)
		expect(screen.getByRole('heading', { name: 'Constats' }).id).toBe(
			'constats',
		)
		const checkbox = screen.getByRole('checkbox', { name: 'Fait' })
		expect((checkbox as HTMLInputElement).checked).toBe(true)
		expect((checkbox as HTMLInputElement).disabled).toBe(true)
		expect(container.querySelectorAll('ul ul')).toHaveLength(1)
		expect(container.querySelector('figcaption')?.textContent).toBe('rust')
		expect(container.querySelector('pre')?.textContent).toBe('let a = 1;')
	})

	it('keeps the compact look without heading ids', () => {
		const { container } = render(<Markdown source={'## Titre\n\n3. a\n4. b'} />)
		expect(screen.getByRole('heading', { name: 'Titre' }).id).toBe('')
		expect(container.querySelector('ol')?.getAttribute('start')).toBe('3')
		expect(container.firstElementChild?.getAttribute('data-variant')).toBe(
			'compact',
		)
	})

	it('drops unsafe link targets', () => {
		render(<Markdown source="[clic](javascript:alert(1))" />)
		expect(screen.queryByRole('link')).toBeNull()
		expect(screen.getByText('clic')).toBeTruthy()
	})
})
