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
	isPending: boolean
	errorMessage?: string
	onConfirm: () => void
	onClose: () => void
}

export function DisconnectGithubDialog({
	open,
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
					<AlertDialogTitle>Déconnecter GitHub ?</AlertDialogTitle>
					<AlertDialogDescription>
						Les cartes ne pourront plus être confiées à Claude Code tant que
						GitHub n’est pas reconnecté. Les exécutions déjà ouvertes gardent
						leurs pull requests.
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
						{isPending ? 'Déconnexion…' : 'Déconnecter'}
					</Button>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	)
}
