interface JsonDisplayProps {
  data: Record<string, unknown>;
}

function colorForType(value: unknown): string {
  if (value === null) return "text-text-secondary";
  if (typeof value === "string") return "text-success";
  if (typeof value === "number") return "text-accent";
  if (typeof value === "boolean") return "text-warning";
  return "text-text-primary";
}

export function JsonDisplay({ data }: JsonDisplayProps) {
  return (
    <div className="bg-bg-primary rounded-md p-3 text-[11px] font-mono leading-relaxed overflow-x-auto">
      <span className="text-text-secondary">{"{"}</span>
      {Object.entries(data).map(([key, value], i, arr) => (
        <div key={key} className="pl-4">
          <span className="text-accent">&quot;{key}&quot;</span>
          <span className="text-text-secondary">: </span>
          <span className={colorForType(value)}>
            {typeof value === "string"
              ? `"${value}"`
              : String(value ?? "null")}
          </span>
          {i < arr.length - 1 && (
            <span className="text-text-secondary">,</span>
          )}
        </div>
      ))}
      <span className="text-text-secondary">{"}"}</span>
    </div>
  );
}
