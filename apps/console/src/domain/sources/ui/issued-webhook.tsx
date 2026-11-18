import { Check, Copy, TriangleAlert } from 'lucide-react'
import { Button } from '#/components/ui/button'
import type { IssuedSource } from '#/domain/sources/model'
import { useCopyToClipboard } from '#/hooks/use-copy-to-clipboard'

interface Props {
	issued: Pick<IssuedSource, 'token' | 'webhook_url'>
	onDismiss?: () => void
}

function CopyField({ label, value }: { label: string; value: string }) {
	const { copied, copy } = useCopyToClipboard()
	return (
		<div className="grid gap-1">
			<span className="text-xs text-muted-foreground">{label}</span>
			<div className="flex min-w-0 items-center gap-1.5">
				<output
					aria-label={label}
					className="block min-w-0 flex-1 rounded-md border bg-background px-2 py-1.5 font-mono text-[12px] [overflow-wrap:anywhere] select-all"
				>
					{value}
				</output>
				<Button
					type="button"
					size="sm"
					variant="outline"
					onClick={() => copy(value)}
					aria-label={`Copier : ${label}`}
				>
					{copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
					{copied ? 'Copié' : 'Copier'}
				</Button>
			</div>
		</div>
	)
}

/** URL et token d'une source, rendus une seule fois par l'API. */
export function IssuedWebhook({ issued, onDismiss }: Props) {
	return (
		<section
			aria-label="Accès au webhook"
			className="grid gap-3 rounded-lg border border-signal-attention-ink/45 bg-signal-attention-ink/8 p-3"
		>
			<p className="m-0 flex items-start gap-2 text-[13px] font-medium text-signal-attention-ink">
				<TriangleAlert aria-hidden="true" className="mt-px size-4 shrink-0" />
				Copie cette URL maintenant : elle ne sera plus affichée. Pour en obtenir
				une nouvelle, il faudra régénérer le token.
			</p>
			<CopyField label="URL du webhook" value={issued.webhook_url} />
			<CopyField label="Token" value={issued.token} />
			<p className="m-0 text-xs text-muted-foreground">
				Envoie un POST JSON sur cette URL. Le titre de la tâche vient du champ{' '}
				<code className="font-mono">title</code>, sinon{' '}
				<code className="font-mono">subject</code>, sinon de la première ligne
				de <code className="font-mono">text</code>.
			</p>
			{onDismiss ? (
				<div>
					<Button type="button" variant="outline" onClick={onDismiss}>
						J'ai copié l'URL
					</Button>
				</div>
			) : null}
		</section>
	)
}
