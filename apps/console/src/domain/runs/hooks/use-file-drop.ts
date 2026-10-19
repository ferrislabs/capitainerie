import { type DragEvent, useState } from 'react'

export function useFileDrop(
	onFile: (file: File) => void,
	onReject: (message: string) => void,
	disabled: boolean,
) {
	const [isDragging, setIsDragging] = useState(false)

	return {
		isDragging,
		handlers: {
			onDragEnter(event: DragEvent<HTMLElement>) {
				event.preventDefault()
				if (!disabled) setIsDragging(true)
			},
			onDragOver(event: DragEvent<HTMLElement>) {
				event.preventDefault()
			},
			onDragLeave(event: DragEvent<HTMLElement>) {
				const next = event.relatedTarget
				if (!(next instanceof Node) || !event.currentTarget.contains(next)) {
					setIsDragging(false)
				}
			},
			onDrop(event: DragEvent<HTMLElement>) {
				event.preventDefault()
				setIsDragging(false)
				if (disabled) return
				const files = event.dataTransfer.files
				if (files.length > 1) {
					onReject('Un seul fichier à la fois')
					return
				}
				const file = files[0]
				if (file) onFile(file)
			},
		},
	}
}
