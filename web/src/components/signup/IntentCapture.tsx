interface IntentCaptureProps {
  intent: string;
  customText: string;
  setIntent: (intent: string) => void;
  setCustomText: (text: string) => void;
  onContinue: () => void;
}

const intentOptions = [
  { key: "claude_token_tracking", label: "Track my Claude Code token spend" },
  { key: "apple_data_unlock", label: "Unlock my Apple data (Podcasts, Notes, Photos)" },
  { key: "replace_cron_jobs", label: "Replace my cron jobs with guaranteed delivery" },
  { key: "ai_agent_infra", label: "Feed my self-hosted AI agents with local data" },
  { key: "google_sheets", label: "Push Mac data to a Google Sheet" },
  { key: "something_else", label: "Something else" },
];

export default function IntentCapture({
  intent,
  customText,
  setIntent,
  setCustomText,
  onContinue,
}: IntentCaptureProps) {
  const handleContinue = () => {
    localStorage.setItem("localpush_signup_intent", intent);
    if (customText) {
      localStorage.setItem("localpush_signup_custom_text", customText);
    }
    onContinue();
  };

  return (
    <div>
      <h2 className="text-2xl font-bold mb-6">Early Access</h2>
      <p className="text-sm font-medium text-text-muted mb-4 tracking-wide uppercase">
        What are you most excited to try?
      </p>

      <div className="space-y-3 mb-6">
        {intentOptions.map((option) => (
          <label
            key={option.key}
            className="flex items-center gap-3 p-4 bg-white/5 rounded-lg border border-white/5 cursor-pointer hover:bg-primary/5 hover:border-primary/20 transition-colors"
          >
            <input
              type="radio"
              name="intent"
              value={option.key}
              checked={intent === option.key}
              onChange={(e) => setIntent(e.target.value)}
              className="w-4 h-4 text-primary accent-primary"
            />
            <span className="text-sm">{option.label}</span>
          </label>
        ))}
      </div>

      {intent === "something_else" && (
        <input
          type="text"
          value={customText}
          onChange={(e) => setCustomText(e.target.value)}
          placeholder="Tell us more..."
          className="mt-2 w-full bg-bg-elevated border border-white/10 rounded-lg text-sm px-4 py-2 text-white placeholder:text-text-muted focus:outline-none focus:ring-1 focus:ring-primary mb-6"
        />
      )}

      <button
        onClick={handleContinue}
        disabled={!intent}
        className="w-full py-3 bg-primary text-bg-deep font-bold rounded-lg hover:bg-primary/90 transition-all disabled:opacity-30 disabled:cursor-not-allowed"
      >
        Continue
      </button>
    </div>
  );
}
