import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "../../utils/logger";

export interface ErrorDiagnosis {
  category:
    | "auth_invalid"
    | "auth_missing"
    | "endpoint_gone"
    | "rate_limited"
    | "target_error"
    | "unreachable"
    | "timeout"
    | "auth_not_configured"
    | "unknown";
  user_message: string;
  guidance: string;
  risk_summary: string | null;
}

export interface RetryAttempt {
  at: number; // unix timestamp
  error: string;
  attempt: number;
}

async function getErrorDiagnosis(entryId: string): Promise<ErrorDiagnosis> {
  logger.debug("Fetching error diagnosis", { entryId });
  try {
    const result = await invoke<ErrorDiagnosis>("get_error_diagnosis", {
      entryId,
    });
    logger.debug("Error diagnosis fetched", {
      entryId,
      category: result.category,
    });
    return result;
  } catch (error) {
    logger.error("Failed to fetch error diagnosis", { entryId, error });
    throw error;
  }
}

async function getRetryHistory(entryId: string): Promise<RetryAttempt[]> {
  logger.debug("Fetching retry history", { entryId });
  try {
    const result = await invoke<RetryAttempt[]>("get_retry_history", {
      entryId,
    });
    logger.debug("Retry history fetched", {
      entryId,
      attempts: result.length,
    });
    return result;
  } catch (error) {
    logger.error("Failed to fetch retry history", { entryId, error });
    throw error;
  }
}

async function getDlqCount(): Promise<number> {
  logger.debug("Fetching DLQ count");
  try {
    const result = await invoke<number>("get_dlq_count");
    logger.debug("DLQ count fetched", { count: result });
    return result;
  } catch (error) {
    logger.error("Failed to fetch DLQ count", { error });
    throw error;
  }
}

export function useErrorDiagnosis(entryId: string | null) {
  return useQuery({
    queryKey: ["errorDiagnosis", entryId],
    queryFn: () => getErrorDiagnosis(entryId!),
    enabled: !!entryId,
    staleTime: 30 * 1000, // 30s - errors don't change often
  });
}

export function useRetryHistory(entryId: string | null) {
  return useQuery({
    queryKey: ["retryHistory", entryId],
    queryFn: () => getRetryHistory(entryId!),
    enabled: !!entryId,
    staleTime: 30 * 1000, // 30s
  });
}

export function useDlqCount() {
  return useQuery({
    queryKey: ["dlqCount"],
    queryFn: getDlqCount,
    refetchInterval: 5000, // Poll every 5s to keep badge updated
  });
}
