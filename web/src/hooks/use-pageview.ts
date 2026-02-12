import { useEffect } from "react";
import { useLocation } from "react-router-dom";
import posthog from "@/lib/posthog";

export function usePageview() {
  const location = useLocation();

  useEffect(() => {
    posthog.capture("$pageview", {
      $current_url: window.origin + location.pathname + location.search,
    });
  }, [location.pathname, location.search]);
}
