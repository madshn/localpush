import { useEffect } from "react";
import { Routes, Route } from "react-router-dom";
import { supabase } from "@/lib/supabase";
import LandingPage from "@/pages/LandingPage";
import BlogPage from "@/pages/BlogPage";

export default function App() {
  useEffect(() => {
    if (!supabase) return;
    const sb = supabase;

    const { data: { subscription } } = sb.auth.onAuthStateChange(
      async (event, session) => {
        if (event === "SIGNED_IN" && session?.user) {
          const intent = localStorage.getItem("localpush_signup_intent");
          const customText = localStorage.getItem("localpush_signup_custom_text");

          if (intent) {
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
