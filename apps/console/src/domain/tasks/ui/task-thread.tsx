import { Check, ChevronDown, Copy, GitPullRequestDraft } from 'lucide-react'
import { type ReactNode, useId, useLayoutEffect, useRef, useState } from 'react'
import { Markdown } from '#/components/markdown'
import { relativeAge } from '#/domain/dashboard/format'
import { pullRequestLink } from '#/domain/tasks/pull-request'
import { AgentAvatar } from '#/domain/tasks/ui/agent-chip'
import { useCopyToClipboard } from '#/hooks/use-copy-to-clipboard'
import { cn } from '#/lib/utils'

/** Au-delà de cette hauteur, un message est replié derrière « Afficher plus ». */
const FOLD_PX = 360

export type Speaker =
	| { kind: 'agent'; name: string; hue: number }
	| { kind: 'human'; name: string; mine: boolean }

function initials(name: string): string {
	const parts = name.trim().split(/\s+/).filter(Boolean)
	const letters =
		parts.length > 1 ? parts[0][0] + parts[1][0] : name.slice(0, 2)
	return letters.toUpperCase()
}

function SpeakerAvatar({ speaker }: { speaker: Speaker }) {
	if (speaker.kind === 'agent') {
		return <AgentAvatar hue={speaker.hue} size={28} />
	}
	return (
		<span
			aria-hidden="true"
			className={cn(
				'inline-grid size-7 shrink-0 place-items-center rounded-full text-[11px] font-semibold',
				speaker.mine
					? 'bg-primary text-primary-foreground'
					: 'bg-secondary text-muted-foreground',
			)}
		>
			{initials(speaker.name)}
		</span>
	)
}

function Foldable({ children }: { children: ReactNode }) {
	const body = useRef<HTMLDivElement>(null)
	const id = useId()
	const [tall, setTall] = useState(false)
	const [open, setOpen] = useState(false)

	useLayoutEffect(() => {
		const node = body.current
		if (!node) return
		const measure = () => setTall(node.scrollHeight > FOLD_PX + 80)
		measure()
		const observer =
			typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure)
		observer?.observe(node)
		return () => observer?.disconnect()
	}, [])

	const folded = tall && !open
	return (
		<>
			<div
				id={id}
				ref={body}
				className={cn(
					'relative',
					folded &&
						'overflow-hidden [mask-image:linear-gradient(to_bottom,black_70%,transparent)]',
				)}
				style={folded ? { maxHeight: FOLD_PX } : undefined}
			>
				{children}
			</div>
			{tall ? (
				<button
					type="button"
					aria-expanded={open}
					aria-controls={id}
					onClick={() => setOpen(!open)}
					className="mx-auto mt-1 flex items-center gap-1 rounded-md px-2 py-1 text-[13px] font-medium text-muted-foreground hover:bg-secondary hover:text-foreground"
				>
					{open ? 'Afficher moins' : 'Afficher plus'}
					<ChevronDown
						aria-hidden="true"
						className={cn(
							'size-3.5 transition-transform',
							open && 'rotate-180',
						)}
					/>
				</button>
			) : null}
		</>
	)
}

function CopyButton({ text }: { text: string }) {
	const { copied, copy } = useCopyToClipboard()
	const Icon = copied ? Check : Copy
	return (
		<button
			type="button"
			aria-label={copied ? 'Copié' : 'Copier le message'}
			title={copied ? 'Copié' : 'Copier'}
			onClick={() => copy(text)}
			className="grid size-7 place-items-center rounded-md text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 hover:bg-secondary hover:text-foreground focus-visible:opacity-100"
		>
			<Icon aria-hidden="true" className="size-3.5" />
		</button>
	)
}

function PullRequestChip({ body }: { body: string }) {
	const pr = pullRequestLink(body)
	if (!pr) return null
	return (
		<a
			href={pr.url}
			target="_blank"
			rel="noopener noreferrer"
			className="mt-2 inline-flex items-center gap-1.5 rounded-md border bg-card px-2.5 py-1 text-[13px] font-medium no-underline! hover:bg-secondary"
		>
			<GitPullRequestDraft aria-hidden="true" className="size-3.5" />
			Ouvrir la pull request {pr.repository}#{pr.number}
		</a>
	)
}

interface MessageProps {
	speaker: Speaker
	body: string
	at: string
	/** Le texte brut de la description, sans Markdown. */
	plain?: boolean
}

/**
 * Un message du fil : avatar et nom, puis le texte. Un agent parle à plat,
 * une personne dans un encadré discret ; un long message se replie.
 */
export function ThreadMessage({
	speaker,
	body,
	at,
	plain = false,
}: MessageProps) {
	const agent = speaker.kind === 'agent'
	return (
		<li className="group grid grid-cols-[28px_minmax(0,1fr)] gap-x-3">
			<SpeakerAvatar speaker={speaker} />
			<div className="min-w-0">
				<p className="m-0 flex h-7 items-center gap-2">
					<b className="font-semibold">{speaker.name}</b>
					<time
						dateTime={at}
						title={new Date(at).toLocaleString('fr-FR')}
						className="text-xs text-muted-foreground"
					>
						{relativeAge(at)}
					</time>
				</p>
				<div
					className={cn(
						'mt-1 text-[14.5px] leading-relaxed',
						!agent && 'rounded-xl border bg-card px-4 py-3',
					)}
				>
					<Foldable>
						{plain ? (
							<p className="m-0 whitespace-pre-wrap">{body}</p>
						) : (
							<Markdown source={body} />
						)}
					</Foldable>
					<PullRequestChip body={body} />
				</div>
				<div className="mt-0.5 flex h-7 items-center">
					<CopyButton text={body} />
				</div>
			</div>
		</li>
	)
}

export function Thread({ children }: { children: ReactNode }) {
	return (
		<ol aria-label="Fil de la tâche" className="m-0 grid list-none gap-4 p-0">
			{children}
		</ol>
	)
}
