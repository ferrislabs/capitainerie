import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { Markdown, parseBlocks, safeHref } from '#/components/markdown'

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

	it('drops unsafe link targets', () => {
		render(<Markdown source="[clic](javascript:alert(1))" />)
		expect(screen.queryByRole('link')).toBeNull()
		expect(screen.getByText('clic')).toBeTruthy()
	})
})
