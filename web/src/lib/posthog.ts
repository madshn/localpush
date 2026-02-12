import posthog from "posthog-js";

const POSTHOG_KEY = "phc_O5u5Q9YfHKY99NeikiVRJ5oPCvjxsp92atHgMWUXZ61";
const POSTHOG_HOST = "https://ph.localpush.app";

posthog.init(POSTHOG_KEY, {
  api_host: POSTHOG_HOST,
  ui_host: "https://eu.posthog.com",
  capture_pageview: false, // We track manually on route change
  capture_pageleave: true,
  autocapture: true,
});

posthog.register({ product_id: "localpush" });

export default posthog;
