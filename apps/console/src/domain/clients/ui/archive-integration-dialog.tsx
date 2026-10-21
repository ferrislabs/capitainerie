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

interface Props {
	open: boolean
	instance?: Schemas.IntegrationInstanceResponse
	isPending: boolean
	errorMessage?: string
	onConfirm: () => void
	onClose: () => void
}

export function ArchiveIntegrationDialog({
	open,
	instance,
	isPending,
	errorMessage,
	onConfirm,
	onClose,
}: Props) {
	return (
		<AlertDialog
			open={open}
			onOpenChange={(next) => {
				if (!next && !isPending) onClose()
			}}
		>
			<AlertDialogContent>
				<AlertDialogHeader>
					<AlertDialogTitle>Archiver l’intégration ?</AlertDialogTitle>
					<AlertDialogDescription>
						{instance
							? `« ${instance.name} » sera archivée et retirée de la liste.`
							: null}
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
						{isPending ? 'Archivage…' : 'Archiver'}
					</Button>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	)
}
