export interface SourcePreview {
  title: string;
  summary: string;
  fields: Array<{ label: string; value: string; sensitive: boolean }>;
  lastUpdated: string | null;
}

export interface DeliveryStatus {
  overall: "active" | "success" | "pending" | "failed" | "unknown";
  pending_count: number;
  failed_count: number;
  last_delivery: string | null;
}

export type FlowStep =
  | "idle"
  | "preview"
  | "pick_endpoint"
  | "configure_delivery"
  | "security_check";

export type DeliveryMode = "on_change" | "daily" | "weekly";

export interface FlowState {
  sourceId: string;
  step: FlowStep;
  preview: SourcePreview | null;
  selectedTarget: string | null;
  selectedEndpoint: string | null;
  selectedEndpointUrl: string | null;
  selectedEndpointName: string | null;
  selectedAuthenticated: boolean;
  selectedAuthType: string | null;
  customHeaders: [string, string][];
  authHeaderName: string;
  authHeaderValue: string;
  isEditing: boolean;
  existingAuthCredentialKey: string | null;
  deliveryMode: DeliveryMode;
  scheduleTime: string | undefined;
  scheduleDay: string | undefined;
}

export const defaultFlowState = (sourceId: string): FlowState => ({
  sourceId,
  step: "idle",
  preview: null,
  selectedTarget: null,
  selectedEndpoint: null,
  selectedEndpointUrl: null,
  selectedEndpointName: null,
  selectedAuthenticated: false,
  selectedAuthType: null,
  customHeaders: [],
  authHeaderName: "",
  authHeaderValue: "",
  isEditing: false,
  existingAuthCredentialKey: null,
  deliveryMode: "on_change",
  scheduleTime: undefined,
  scheduleDay: undefined,
});

export type SourceCategory = "active" | "paused" | "available";

export interface SourceData {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  last_sync: string | null;
  watch_path: string | null;
}

export interface SourceWithCategory {
  source: SourceData;
  category: SourceCategory;
}

export type TrafficLightStatus = "green" | "yellow" | "red" | "grey";
