import { Plus } from "lucide-react";

interface AddTargetCardProps {
  onClick: () => void;
}

export function AddTargetCard({ onClick }: AddTargetCardProps) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-2 px-3 py-2 border border-dashed border-border rounded-lg opacity-40 hover:opacity-70 hover:border-accent transition-all cursor-pointer min-w-0"
    >
      <Plus size={16} className="shrink-0 text-text-secondary" />
      <span className="text-xs font-medium text-text-secondary">
        Add Target
      </span>
    </button>
  );
}
