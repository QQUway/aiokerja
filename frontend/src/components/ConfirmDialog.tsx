import Win95Dialog from "./Win95Dialog";

interface ConfirmDialogProps {
  title?: string;
  message: string;
  confirmLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function ConfirmDialog({
  title = "Confirm",
  message,
  confirmLabel = "OK",
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  return (
    <Win95Dialog
      title={title}
      icon="warning"
      onClose={onCancel}
      actions={
        <>
          <button className="primary" onClick={onConfirm}>
            {confirmLabel}
          </button>
          <button onClick={onCancel}>Cancel</button>
        </>
      }
    >
      {message}
    </Win95Dialog>
  );
}
