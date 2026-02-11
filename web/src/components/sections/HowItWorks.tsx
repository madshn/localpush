import { Eye, Search, Link, Clock } from 'lucide-react';

export default function HowItWorks() {
  const steps = [
    {
      number: '01',
      icon: Eye,
      title: 'Watch',
      description:
        'Sources: Claude Code Stats, Apple Podcasts, Apple Notes, Apple Photos — and growing. LocalPush detects changes the moment they happen.',
    },
    {
      number: '02',
      icon: Search,
      title: 'Preview',
      description:
        'Before anything is sent, you see YOUR actual data. Not samples. Not descriptions. Your real token counts, your real podcast history.',
    },
    {
      number: '03',
      icon: Link,
      title: 'Connect',
      description:
        'Connect your n8n, Zapier, Make, or Google Sheets receiver. Data delivered securely with zero data loss. WAL-backed queue survives crashes, reboots, and outages.',
      platforms: ['N8N', 'GS', 'ZP', 'Make'],
    },
    {
      number: '04',
      icon: Clock,
      title: 'Your Cadence',
      description:
        'Push in real-time, daily digest, or weekly digest — you choose per source.',
      cadences: ['Real-time', 'Daily', 'Weekly'],
    },
  ];

  return (
    <section className="py-24 border-b border-border-muted">
      <div className="container mx-auto px-6 lg:px-12">
        {/* Header */}
        <div className="text-center mb-16">
          <h2 className="text-4xl md:text-5xl font-bold mb-4 font-display">
            How it Works
          </h2>
          <p className="text-lg text-text-muted max-w-2xl mx-auto">
            Streamline your local data automation in four simple steps.
          </p>
        </div>

        {/* Four-column grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8">
          {steps.map((step, i) => {
            const Icon = step.icon;
            return (
              <div
                key={i}
                className="p-8 rounded-xl bg-white/[0.02] border border-border-muted hover:border-primary/50 transition-all"
              >
                {/* Step label */}
                <div className="font-mono text-[10px] text-primary uppercase tracking-widest mb-4">
                  Step {step.number}
                </div>

                {/* Icon */}
                <div className="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center mb-6">
                  <Icon className="w-6 h-6 text-primary" />
                </div>

                {/* Title */}
                <h3 className="text-xl font-bold mb-3 font-display">
                  {step.title}
                </h3>

                {/* Description */}
                <p className="text-text-muted text-sm leading-relaxed mb-4">
                  {step.description}
                </p>

                {/* Platform badges (step 3) */}
                {step.platforms && (
                  <div className="flex flex-wrap gap-2 mt-4">
                    {step.platforms.map((platform, j) => (
                      <div
                        key={j}
                        className="px-3 py-1.5 rounded bg-white/5 border border-white/10 text-xs font-mono text-text-muted opacity-60"
                      >
                        {platform}
                      </div>
                    ))}
                  </div>
                )}

                {/* Cadence labels (step 4) */}
                {step.cadences && (
                  <div className="flex flex-wrap gap-2 mt-4">
                    {step.cadences.map((cadence, j) => (
                      <div
                        key={j}
                        className={`px-3 py-1.5 rounded bg-white/5 border border-white/10 text-xs font-mono ${
                          cadence === 'Real-time'
                            ? 'text-primary font-bold'
                            : 'text-text-muted'
                        }`}
                      >
                        {cadence}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
