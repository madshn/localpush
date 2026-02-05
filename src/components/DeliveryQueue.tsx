import { useDeliveryQueue } from "../api/hooks/useDeliveryQueue";

export function DeliveryQueue() {
  const { data: queue, isLoading } = useDeliveryQueue();

  const handleRetry = async (eventId: string) => {
    // TODO: Implement retry_delivery command in Rust backend
    alert("Retry functionality not yet implemented in backend");
    console.log("Would retry event:", eventId);
  };

  if (isLoading) {
    return <div>Loading queue...</div>;
  }

  const pending = queue?.filter((item) => item.status === "pending") ?? [];
  const inFlight = queue?.filter((item) => item.status === "in_flight") ?? [];
  const failed = queue?.filter((item) => item.status === "failed") ?? [];

  return (
    <div>
      <div className="card">
        <h2 className="card-title">Delivery Queue</h2>
        <div className="queue-stats">
          <div>
            <span className="status-dot active" /> Delivered today:{" "}
            {queue?.filter((item) => item.status === "delivered").length ?? 0}
          </div>
          <div>
            <span className="status-dot pending" /> Pending: {pending.length}
          </div>
          <div>
            <span className="status-dot pending" /> In flight: {inFlight.length}
          </div>
          {failed.length > 0 && (
            <div>
              <span className="status-dot error" /> Failed: {failed.length}
            </div>
          )}
        </div>
      </div>

      {failed.length > 0 && (
        <div className="card">
          <h2 className="card-title">Failed Deliveries</h2>
          {failed.map((item) => (
            <div key={item.id} className="source-item">
              <div className="source-info">
                <h3>{item.eventType}</h3>
                <p>{item.lastError}</p>
                <p>Attempts: {item.retryCount}</p>
              </div>
              <button className="btn" onClick={() => handleRetry(item.id)}>
                Retry
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
