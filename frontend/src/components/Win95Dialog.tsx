import type { ReactNode } from "react";

const ICONS: Record<string, string> = {
  warning: "⚠",
  info: "ℹ",
  error: "✕",
  question: "?",
};

interface Win95DialogProps {
  title: string;
  children: ReactNode;
  onClose: () => void;
  actions?: ReactNode;
  icon?: keyof typeof ICONS;
}

export default function Win95Dialog({ title, children, onClose, actions, icon }: Win95DialogProps) {
  return (
    <div
      className="win95-overlay"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="window win95-dialog" role="dialog" aria-modal="true" aria-label={title}>
        <div className="title-bar">
          <div className="title-bar-text">{title}</div>
          <div className="title-bar-controls">
            <button aria-label="Close" onClick={onClose} />
          </div>
        </div>
        <div className="window-body win95-dialog-body">
          {icon && (
            <div className="win95-dialog-icon" aria-hidden="true">
              {ICONS[icon]}
            </div>
          )}
          <div className="win95-dialog-content">{children}</div>
        </div>
        {actions && <div className="win95-dialog-actions">{actions}</div>}
      </div>
    </div>
  );
}
