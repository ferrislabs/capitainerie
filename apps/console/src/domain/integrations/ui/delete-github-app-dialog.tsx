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
	htmlUrl?: string
	isPending: boolean
	errorMessage?: string
	onConfirm: () => void
	onClose: () => void
}

export function DeleteGithubAppDialog({
	open,
	htmlUrl,
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
					<AlertDialogTitle>Supprimer l’application GitHub ?</AlertDialogTitle>
					<AlertDialogDescription>
						Capitainerie oubliera l’application et son installation : les cartes
						ne pourront plus être confiées à Claude Code tant qu’une nouvelle
						application n’est pas créée. L’application reste sur GitHub :
						supprimez-la aussi dans ses réglages
						{htmlUrl ? (
							<>
								{' '}
								(
								<a
									href={htmlUrl}
									target="_blank"
									rel="noreferrer"
									className="underline underline-offset-2"
								>
									ouvrir la page de l’application
								</a>
								)
							</>
						) : null}
						. Les exécutions déjà ouvertes gardent leurs pull requests.
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
						{isPending ? 'Suppression…' : 'Supprimer l’application'}
					</Button>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	)
}
