import { Button } from '#/components/ui/button'

interface Props {
	onCancel: () => void
	onSave: () => void
}

export function SaveBar({ onCancel, onSave }: Props) {
	return (
		<div className="sticky bottom-0 z-10 mt-3 flex items-center gap-2 rounded-[9px] border bg-secondary px-3 py-[9px] shadow-[0_-8px_20px_rgba(0,0,0,0.25)]">
			<span>Modifications non enregistrées</span>
			<span className="flex-1" />
			<Button type="button" variant="outline" onClick={onCancel}>
				Annuler
			</Button>
			<Button type="button" onClick={onSave}>
				Enregistrer
			</Button>
		</div>
	)
}
