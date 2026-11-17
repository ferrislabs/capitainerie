import { FlaskConical } from 'lucide-react'

/** Bandeau des pages fournies avec des données de démonstration : rien n'est enregistré. */
export function DemoBanner() {
	return (
		<output className="flex items-center gap-2 rounded-lg border border-signal-attention/40 bg-signal-attention-soft px-3 py-2 text-signal-attention-ink">
			<FlaskConical aria-hidden="true" className="size-4 shrink-0" />
			<span>
				<b className="font-medium">Données de démonstration.</b> Cette page
				n'est pas encore reliée au backend : rien n'est enregistré et tout est
				perdu au rechargement.
			</span>
		</output>
	)
}
