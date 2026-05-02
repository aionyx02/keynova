import { useEffect, useRef, type ReactNode } from "react";

interface FloatingWindowProps {
  title: string;
  children: ReactNode;
  onClose: () => void;
  className?: string;
}

export function FloatingWindow({ title, children, onClose, className = "" }: FloatingWindowProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      ref={ref}
      className={`fixed inset-0 flex items-center justify-center bg-black/50 z-50 ${className}`}
    >
      <div className="bg-gray-900 border border-gray-700 rounded-lg shadow-2xl w-[600px] max-h-[80vh] flex flex-col overflow-hidden">
        <div className="flex items-center justify-between px-4 py-2 border-b border-gray-700 select-none">
          <span className="text-sm font-medium text-gray-300">{title}</span>
          <button
            onClick={onClose}
            className="text-gray-500 hover:text-gray-200 text-lg leading-none"
            aria-label="Close"
          >
            ×
          </button>
        </div>
        <div className="flex-1 overflow-auto">{children}</div>
      </div>
    </div>
  );
}
