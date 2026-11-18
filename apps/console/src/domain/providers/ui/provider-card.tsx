import { Check, KeyRound, Pencil, Star, Trash2, Wrench } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { Button } from '#/components/ui/button'
import { Sev } from '#/domain/audit/ui/sev'
import { maskedPreview, type ProviderSpec } from '#/domain/providers/model'

export type TestState = { ok: true } | { ok: false; message: string }

interface Props {
	spec: ProviderSpec
	provider: Schemas.UserProviderResponse | undefined
	busy: boolean
	test: TestState | undefined
	onEdit: (spec: ProviderSpec) => void
	onTest: (spec: ProviderSpec) => void
	onDelete: (spec: ProviderSpec) => void
	onMakeDefault: (provider: Schemas.UserProviderResponse) => void
}

export function ProviderCard({
	spec,
	provider,
	busy,
	test,
	onEdit,
	onTest,
	onDelete,
	onMakeDefault,
}: Props) {
	return (
		<article
			aria-label={spec.name}
			className="flex min-w-0 flex-col gap-2 overflow-hidden rounded-lg border bg-card px-3 py-[11px]"
		>
			<div className="flex flex-wrap items-center gap-2">
				<h3 className="m-0 text-[13px] font-semibold">{spec.name}</h3>
				<Sev tone={provider ? 'ok' : 'info'}>
					{provider ? 'Connecté' : 'Non connecté'}
				</Sev>
				{provider?.is_default ? <Sev tone="vio">Par défaut</Sev> : null}
			</div>
			{provider ? (
				<dl className="m-0 grid gap-0.5 text-xs text-muted-foreground">
					<div className="flex gap-1.5">
						<dt>Clé</dt>
						<dd className="m-0 font-mono">{maskedPreview(provider.preview)}</dd>
					</div>
					<div className="flex gap-1.5">
						<dt>Modèle par défaut</dt>
						<dd className="m-0 font-mono">{provider.default_model}</dd>
					</div>
					{provider.base_url ? (
						<div className="flex min-w-0 gap-1.5">
							<dt>URL</dt>
							<dd className="m-0 min-w-0 truncate font-mono">
								{provider.base_url}
							</dd>
						</div>
					) : null}
				</dl>
			) : null}
			<div className="flex flex-wrap gap-1.5 border-t pt-[7px]">
				{provider ? (
					<>
						<Button
							variant="outline"
							size="sm"
							disabled={busy}
							onClick={() => onEdit(spec)}
						>
							<Pencil aria-hidden="true" />
							Modifier
						</Button>
						<Button
							variant="outline"
							size="sm"
							disabled={busy}
							onClick={() => onTest(spec)}
						>
							<Wrench aria-hidden="true" />
							Tester
						</Button>
						{provider.is_default ? null : (
							<Button
								variant="outline"
								size="sm"
								disabled={busy}
								onClick={() => onMakeDefault(provider)}
							>
								<Star aria-hidden="true" />
								Définir par défaut
							</Button>
						)}
						<Button
							variant="outline"
							size="sm"
							disabled={busy}
							onClick={() => onDelete(spec)}
						>
							<Trash2 aria-hidden="true" />
							Supprimer
						</Button>
					</>
				) : (
					<Button size="sm" disabled={busy} onClick={() => onEdit(spec)}>
						<KeyRound aria-hidden="true" />
						Connecter
					</Button>
				)}
			</div>
			{test ? (
				test.ok ? (
					<p className="m-0 flex items-center gap-1.5 border-t pt-[7px] text-signal-success">
						<Check className="size-3.5" aria-hidden="true" />
						Connexion réussie : le fournisseur répond.
					</p>
				) : (
					<p role="alert" className="m-0 border-t pt-[7px] text-signal-danger">
						{test.message}
					</p>
				)
			) : null}
		</article>
	)
}
