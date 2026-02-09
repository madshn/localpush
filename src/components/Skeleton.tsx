interface SkeletonProps {
  className?: string;
}

export function Skeleton({ className = "" }: SkeletonProps) {
  return <div className={`skeleton ${className}`} />;
}

export function SkeletonCard() {
  return (
    <div className="bg-bg-secondary border border-border rounded-lg p-4">
      <div className="flex items-center gap-3 mb-3">
        <Skeleton className="w-1 h-8" />
        <div className="flex-1">
          <Skeleton className="h-3.5 w-24 mb-2" />
          <Skeleton className="h-2.5 w-16" />
        </div>
        <Skeleton className="h-7 w-16 rounded-md" />
      </div>
    </div>
  );
}
