import { useEffect } from "react";
import { Routes, Route } from "react-router-dom";
import { supabase } from "@/lib/supabase";
import posthog from "@/lib/posthog";
import { usePageview } from "@/hooks/use-pageview";
import LandingPage from "@/pages/LandingPage";
import BlogPage from "@/pages/BlogPage";

export default function App() {
  usePageview();

  useEffect(() => {
    if (!supabase) return;
    const sb = supabase;

    const { data: { subscription } } = sb.auth.onAuthStateChange(
      async (event, session) => {
        if (event === "SIGNED_IN" && session?.user) {
          posthog.identify(session.user.id, {
            email: session.user.email,
            provider: session.user.app_metadata.provider ?? "unknown",
            username:
              session.user.user_metadata.user_name ??
              session.user.user_metadata.full_name ??
              null,
          });

          const intent = localStorage.getItem("localpush_signup_intent");
          const customText = localStorage.getItem("localpush_signup_custom_text");

          if (intent) {
            posthog.capture("signup_completed", {
              product_id: "localpush",
              method: session.user.app_metadata.provider ?? "unknown",
              intent,
              custom_text: customText || null,
            });

            await sb.from("localpush_site_users").upsert({
              id: session.user.id,
              provider: session.user.app_metadata.provider ?? "unknown",
              email: session.user.email,
              username:
                session.user.user_metadata.user_name ??
                session.user.user_metadata.full_name ??
                null,
              avatar_url: session.user.user_metadata.avatar_url ?? null,
              intent,
              custom_intent_text: customText || null,
            });

            localStorage.removeItem("localpush_signup_intent");
            localStorage.removeItem("localpush_signup_custom_text");
          }
        }
      }
    );

    return () => subscription.unsubscribe();
  }, []);

  return (
    <Routes>
      <Route path="/" element={<LandingPage />} />
      <Route path="/blog" element={<BlogPage />} />
    </Routes>
  );
}
