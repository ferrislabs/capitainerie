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
import { PROVIDER_INFO, type ProviderKind } from '#/domain/settings/providers'

interface Props {
	open: boolean
	kind: ProviderKind | undefined
	isPending: boolean
	errorMessage?: string
	onConfirm: () => void
	onClose: () => void
}

export function DeleteProviderDialog({
	open,
	kind,
	isPending,
	errorMessage,
	onConfirm,
	onClose,
}: Props) {
	const label = kind ? PROVIDER_INFO[kind].label : ''

	return (
		<AlertDialog
			open={open && kind !== undefined}
			onOpenChange={(next) => {
				if (!next && !isPending) onClose()
			}}
		>
			<AlertDialogContent>
				<AlertDialogHeader>
					<AlertDialogTitle>Supprimer {label} ?</AlertDialogTitle>
					<AlertDialogDescription>
						La clé est effacée. Les rôles qui utilisent ce fournisseur ne
						pourront plus lancer de run tant qu’un autre modèle ne leur est pas
						assigné.
					</AlertDialogDescription>
				</AlertDialogHeader>
				{errorMessage ? (
					<InlineAlert tone="danger">{errorMessage}</InlineAlert>
				) : null}
				<AlertDialogFooter>
					<AlertDialogCancel disabled={isPending}>Annuler</AlertDialogCancel>
					<Button
						type="button"
						variant="destructive"
						disabled={isPending}
						onClick={onConfirm}
					>
						{isPending ? 'Suppression…' : 'Supprimer'}
					</Button>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	)
}
