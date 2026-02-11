import { BarChart3, Music, Webhook, Brain, Shield } from 'lucide-react';

export default function UseCases() {
  const cases = [
    {
      icon: BarChart3,
      title: 'Claude Code Tracking',
      description:
        'Track your AI token spend automatically. Session stats flow to your dashboard. Setup time: 5 minutes.',
      color: 'text-blue-400',
    },
    {
      icon: Music,
      title: 'Apple Data Automation',
      description:
        'Podcasts listening history, Notes changes, Photos metadata — flowing into your automation stack.',
      color: 'text-green-400',
    },
    {
      icon: Webhook,
      title: 'File → Webhook',
      description:
        'Watch any file or directory. When it changes, the new data hits your webhook. Replace fragile cron jobs with event-driven, guaranteed delivery.',
      color: 'text-orange-400',
    },
    {
      icon: Brain,
      title: 'Local AI Agent Infrastructure',
      description:
        'Running self-hosted AI agents? Feed them local Mac data safely. LocalPush delivers to your own n8n server with radical transparency. More than a single claw — power an entire agent team.',
      color: 'text-purple-400',
    },
    {
      icon: Shield,
      title: 'Privacy-First Pipeline',
      description:
        "Open source (MIT). Runs locally. No cloud dependency. You see exactly what's sent before it's sent. Audit the code yourself.",
      color: 'text-text-primary',
    },
  ];

  return (
    <section className="py-24 border-b border-border-muted">
      <div className="container mx-auto px-6 lg:px-12">
        {/* Header */}
        <div className="text-center mb-16">
          <h2 className="text-4xl md:text-5xl font-bold font-display">
            What will you unlock?
          </h2>
        </div>

        {/* 3+2 asymmetric grid */}
        <div className="grid grid-cols-1 md:grid-cols-6 gap-6">
          {/* Top 3 cards */}
          {cases.slice(0, 3).map((useCase, i) => {
            const Icon = useCase.icon;
            return (
              <div
                key={i}
                className="md:col-span-2 p-8 rounded-xl bg-white/[0.03] border border-border-muted hover:border-primary/50 transition-all group"
              >
                <div className="w-12 h-12 rounded-lg bg-white/5 flex items-center justify-center mb-6 group-hover:scale-110 transition-transform">
                  <Icon className={`w-6 h-6 ${useCase.color}`} />
                </div>
                <h3 className="text-xl font-bold mb-3 font-display">
                  {useCase.title}
                </h3>
                <p className="text-text-muted leading-relaxed">
                  {useCase.description}
                </p>
              </div>
            );
          })}

          {/* Bottom 2 cards */}
          {cases.slice(3, 5).map((useCase, i) => {
            const Icon = useCase.icon;
            return (
              <div
                key={i + 3}
                className="md:col-span-3 p-8 rounded-xl bg-white/[0.03] border border-border-muted hover:border-primary/50 transition-all group"
              >
                <div className="w-12 h-12 rounded-lg bg-white/5 flex items-center justify-center mb-6 group-hover:scale-110 transition-transform">
                  <Icon className={`w-6 h-6 ${useCase.color}`} />
                </div>
                <h3 className="text-xl font-bold mb-3 font-display">
                  {useCase.title}
                </h3>
                <p className="text-text-muted leading-relaxed">
                  {useCase.description}
                </p>
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
