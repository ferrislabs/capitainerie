import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import {
	AlertDialog,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from '#/components/ui/alert-dialog'
import { Button } from '#/components/ui/button'
import {
	targetSummary,
	toolLabel,
} from '#/domain/integrations/journal/action-labels'

interface Props {
	action: Schemas.IntegrationActionResponse
	isPending: boolean
	errorMessage?: string
	onConfirm: (confirmProduction: boolean) => void
	onClose: () => void
}

export function RevertDialog({
	action,
	isPending,
	errorMessage,
	onConfirm,
	onClose,
}: Props) {
	const [authorised, setAuthorised] = useState(false)
	const production = action.instance_environment === 'production'
	const canConfirm = !isPending && (!production || authorised)

	return (
		<AlertDialog
			open
			onOpenChange={(next) => {
				if (!next && !isPending) onClose()
			}}
		>
			<AlertDialogContent>
				<AlertDialogHeader>
					<AlertDialogTitle>Restaurer l’état d’avant ?</AlertDialogTitle>
					<AlertDialogDescription>
						L’état d’avant sera réécrit sur {action.instance_name}.
					</AlertDialogDescription>
					<p className="font-mono text-xs break-all text-muted-foreground">
						{toolLabel(action.tool)} {targetSummary(action.target)}
					</p>
				</AlertDialogHeader>
				{production ? (
					<label className="flex items-start gap-2 rounded-md border border-signal-danger/35 bg-signal-danger-soft px-3 py-2 text-[13px] font-medium text-signal-danger">
						<input
							type="checkbox"
							required
							className="mt-0.5 size-4 shrink-0 accent-signal-danger"
							checked={authorised}
							disabled={isPending}
							onChange={(event) => setAuthorised(event.target.checked)}
						/>
						J’autorise l’écriture en production
					</label>
				) : null}
				{errorMessage ? (
					<InlineAlert tone="danger">{errorMessage}</InlineAlert>
				) : null}
				<AlertDialogFooter>
					<AlertDialogCancel disabled={isPending}>Annuler</AlertDialogCancel>
					<Button
						type="button"
						disabled={!canConfirm}
						onClick={() => onConfirm(production && authorised)}
					>
						{isPending ? 'Restauration…' : 'Restaurer'}
					</Button>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	)
}
