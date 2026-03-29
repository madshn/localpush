import { Terminal, Podcast, StickyNote, Image, BarChart3, Download } from "lucide-react";
import type { SourceCategory } from "./types";

const sourceIcons: Record<string, { icon: typeof Terminal; color: string }> = {
  "claude-stats": { icon: Terminal, color: "text-accent" },
  "claude-sessions": { icon: Terminal, color: "text-accent" },
  "cic-task-output": { icon: Download, color: "text-sky-400" },
  "apple-podcasts": { icon: Podcast, color: "text-purple-400" },
  "apple-notes": { icon: StickyNote, color: "text-yellow-400" },
  "apple-photos": { icon: Image, color: "text-pink-400" },
};

const fallbackIcon = { icon: BarChart3, color: "text-text-secondary" };

interface SourceCardProps {
  sourceId: string;
  name: string;
  category: SourceCategory;
  nextPushLabel?: string | null;
}

export function SourceCard({ sourceId, name, category, nextPushLabel }: SourceCardProps) {
  const { icon: Icon, color } = sourceIcons[sourceId] || fallbackIcon;
  const dimmed = category === "available" || category === "paused";

  return (
    <div
      className={`flex items-center gap-2 px-3 py-2 bg-bg-secondary border border-border rounded-lg min-w-0 ${dimmed ? "opacity-60" : ""}`}
    >
      <Icon size={16} className={`shrink-0 ${color}`} />
      <div className="min-w-0">
        <div className="text-xs font-medium truncate">{name}</div>
        {nextPushLabel && (
          <div className="text-[10px] text-text-secondary truncate">{nextPushLabel}</div>
        )}
      </div>
    </div>
  );
}
