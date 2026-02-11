import { Github } from 'lucide-react';

interface HeroProps {
  onOpenSignup: () => void;
}

export default function Hero({ onOpenSignup }: HeroProps) {
  return (
    <section className="relative min-h-screen flex items-center border-b border-border-muted overflow-hidden">
      {/* Radial gradient glow */}
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_50%,rgba(59,130,246,0.1),transparent_50%)]" />

      <div className="relative container mx-auto px-6 lg:px-12 py-20">
        <div className="grid lg:grid-cols-2 gap-16 items-center">
          {/* LEFT COLUMN */}
          <div>
            {/* Version badge */}
            <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-white/5 border border-white/10 mb-8">
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2 w-2 bg-primary"></span>
              </span>
              <span className="text-sm font-mono text-text-muted">v0.8.5-beta Available</span>
            </div>

            {/* Headline */}
            <h1 className="text-6xl font-bold leading-[1.1] tracking-tight mb-6 font-display">
              Unlock your<br />
              <span className="text-primary">Mac data.</span>
            </h1>

            {/* Subheadline from BRIEF */}
            <p className="text-xl text-text-muted leading-relaxed max-w-lg mb-10">
              Your Mac stores incredible data — Claude Code usage, Apple Podcasts, Notes, Photos — locked away from your automation stack. LocalPush watches it and pushes changes to n8n, Make, Zapier, or even a Google Sheet with guaranteed delivery. Currently in beta.
            </p>

            {/* CTAs */}
            <div className="flex flex-wrap gap-4">
              <button
                onClick={onOpenSignup}
                className="px-8 py-3.5 bg-primary hover:bg-primary/90 text-bg-deep font-semibold rounded-lg transition-all"
              >
                Become an Early Tester
              </button>
              <a
                href="https://github.com/madshn/localpush"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-2 px-8 py-3.5 bg-white/5 hover:bg-white/10 text-text-primary font-semibold rounded-lg border border-white/10 transition-all"
              >
                <Github className="w-5 h-5" />
                Star on GitHub →
              </a>
            </div>
          </div>

          {/* RIGHT COLUMN - Mock UI */}
          <div className="relative">
            {/* Blue glow behind card */}
            <div className="absolute inset-0 bg-primary/20 blur-[100px] scale-75" />

            {/* Mock menu bar popup */}
            <div className="relative bg-bg-surface rounded-xl border border-white/10 shadow-2xl overflow-hidden">
              {/* Fake window chrome */}
              <div className="bg-[#121212] px-4 py-3 flex items-center gap-2 border-b border-white/5">
                <div className="w-3 h-3 rounded-full bg-white/20" />
                <div className="w-3 h-3 rounded-full bg-white/20" />
                <div className="w-3 h-3 rounded-full bg-white/20" />
              </div>

              {/* Status header */}
              <div className="px-6 py-4 border-b border-white/5">
                <h3 className="text-sm font-semibold text-text-primary">LocalPush Status</h3>
              </div>

              {/* Source rows */}
              <div className="p-4 space-y-3">
                {[
                  { name: 'Claude Code Stats', time: '2m ago' },
                  { name: 'Apple Podcasts', time: '14m ago' },
                  { name: 'Apple Notes', time: '5m ago' },
                  { name: 'Apple Photos', time: 'Just now' },
                ].map((source, i) => (
                  <div key={i} className="flex items-center justify-between px-4 py-3 rounded-lg bg-white/[0.02] hover:bg-white/[0.04] transition-colors">
                    <div className="flex items-center gap-3">
                      <div className="w-2 h-2 rounded-full bg-success" />
                      <span className="text-sm font-medium text-text-primary">{source.name}</span>
                    </div>
                    <span className="text-xs text-text-muted font-mono">{source.time}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
