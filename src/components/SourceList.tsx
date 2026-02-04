import { useSources } from "../api/hooks/useSources";

export function SourceList() {
  const { data: sources, isLoading } = useSources();

  if (isLoading) {
    return <div>Loading sources...</div>;
  }

  return (
    <div className="card">
      <h2 className="card-title">Data Sources</h2>
      {sources?.map((source) => (
        <div key={source.id} className="source-item">
          <div className="source-info">
            <h3>{source.name}</h3>
            <p>{source.description}</p>
            {source.lastSync && (
              <p>Last sync: {new Date(source.lastSync).toLocaleString()}</p>
            )}
          </div>
          <button className={source.enabled ? "btn btn-secondary" : "btn"}>
            {source.enabled ? "Disable" : "Enable"}
          </button>
        </div>
      ))}
    </div>
  );
}
