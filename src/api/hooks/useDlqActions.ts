import { useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { logger } from "../../utils/logger";

async function dismissDlqEntry(entryId: string): Promise<void> {
  logger.debug("Dismissing DLQ entry", { entryId });
  try {
    await invoke("dismiss_dlq_entry", { entryId });
    logger.info("DLQ entry dismissed", { entryId });
  } catch (error) {
    logger.error("Failed to dismiss DLQ entry", { entryId, error });
    throw error;
  }
}

async function replayDelivery(entryId: string): Promise<void> {
  logger.debug("Replaying delivery", { entryId });
  try {
    await invoke("replay_delivery", { entryId });
    logger.info("Delivery replayed", { entryId });
  } catch (error) {
    logger.error("Failed to replay delivery", { entryId, error });
    throw error;
  }
}

export function useDismissDlq() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: dismissDlqEntry,
    onSuccess: (_, entryId) => {
      // Invalidate all queries that might be affected by dismissal
      queryClient.invalidateQueries({ queryKey: ["activityLog"] });
      queryClient.invalidateQueries({ queryKey: ["dlqCount"] });
      queryClient.invalidateQueries({ queryKey: ["deliveryStatus"] });
      toast.success("Failure dismissed");
      logger.info("DLQ entry dismissed successfully", { entryId });
    },
    onError: (error, entryId) => {
      toast.error(`Failed to dismiss: ${error}`);
      logger.error("Failed to dismiss DLQ entry", { entryId, error });
    },
  });
}

export function useReplayDelivery() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: replayDelivery,
    onSuccess: (_, entryId) => {
      // Invalidate queries to show new delivery in queue
      queryClient.invalidateQueries({ queryKey: ["activityLog"] });
      queryClient.invalidateQueries({ queryKey: ["deliveryQueue"] });
      toast.success("Delivery replayed — will send within 5s");
      logger.info("Delivery replayed successfully", { entryId });
    },
    onError: (error, entryId) => {
      toast.error(`Replay failed: ${error}`);
      logger.error("Failed to replay delivery", { entryId, error });
    },
  });
}
