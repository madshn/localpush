import { Webhook, Bell, Zap, Link } from "lucide-react";

const targetIcons: Record<string, { icon: typeof Webhook; color: string }> = {
  n8n: { icon: Webhook, color: "text-orange-400" },
  ntfy: { icon: Bell, color: "text-green-400" },
  make: { icon: Zap, color: "text-violet-400" },
  zapier: { icon: Zap, color: "text-orange-300" },
};

const fallbackIcon = { icon: Link, color: "text-text-secondary" };

interface TargetCardProps {
  targetType: string;
  endpointName: string;
  endpointUrl: string;
}

export function TargetCard({
  targetType,
  endpointName,
  endpointUrl,
}: TargetCardProps) {
  const { icon: Icon, color } = targetIcons[targetType] || fallbackIcon;

  return (
    <div className="flex items-center gap-2 px-3 py-2 bg-bg-secondary border border-border rounded-lg min-w-0">
      <Icon size={16} className={`shrink-0 ${color}`} />
      <div className="min-w-0 flex-1">
        <div className="text-xs font-medium truncate">{endpointName}</div>
        <div className="text-[10px] text-text-secondary font-mono truncate">
          {endpointUrl}
        </div>
      </div>
    </div>
  );
}
