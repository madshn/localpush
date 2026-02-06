interface TrafficLightProps {
  status: "green" | "yellow" | "red" | "grey";
}

export function TrafficLight({ status }: TrafficLightProps) {
  return <div className={`traffic-light traffic-${status}`} />;
}
