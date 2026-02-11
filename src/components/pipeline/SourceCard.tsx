import { Terminal, Podcast, StickyNote, Image, BarChart3 } from "lucide-react";
import type { SourceCategory } from "./types";

const sourceIcons: Record<string, { icon: typeof Terminal; color: string }> = {
  claude_code_stats: { icon: Terminal, color: "text-accent" },
  claude_code_sessions: { icon: Terminal, color: "text-accent" },
  apple_podcasts: { icon: Podcast, color: "text-purple-400" },
  apple_notes: { icon: StickyNote, color: "text-yellow-400" },
  apple_photos: { icon: Image, color: "text-pink-400" },
};

const fallbackIcon = { icon: BarChart3, color: "text-text-secondary" };

interface SourceCardProps {
  sourceId: string;
  name: string;
  category: SourceCategory;
}

export function SourceCard({ sourceId, name, category }: SourceCardProps) {
  const { icon: Icon, color } = sourceIcons[sourceId] || fallbackIcon;
  const dimmed = category === "available" || category === "paused";

  return (
    <div
      className={`flex items-center gap-2 px-3 py-2 bg-bg-secondary border border-border rounded-lg min-w-0 ${dimmed ? "opacity-60" : ""}`}
    >
      <Icon size={16} className={`shrink-0 ${color}`} />
      <span className="text-xs font-medium truncate">{name}</span>
    </div>
  );
}
