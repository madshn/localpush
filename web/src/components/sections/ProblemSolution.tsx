import { X, Check } from 'lucide-react';

export default function ProblemSolution() {
  const problems = [
    'Claude Code token stats locked in ~/.claude/',
    'Apple Podcasts history trapped in SQLite',
    'Notes/Photos metadata invisible',
    'Fragile cron jobs that fail silently',
  ];

  const solutions = [
    'Pushes to n8n/Make/Zapier/Sheets',
    'WAL-backed guaranteed delivery',
    'See YOUR real data before it leaves',
  ];

  return (
    <section className="py-24 border-b border-border-muted">
      <div className="container mx-auto px-6 lg:px-12">
        <div className="grid lg:grid-cols-2 gap-16 lg:gap-24">
          {/* LEFT - Problem */}
          <div>
            <div className="text-problem text-lg font-mono font-medium tracking-tight uppercase mb-4">
              The Problem
            </div>
            <h2 className="text-4xl font-bold mb-6 font-display">
              Your Mac is full of data you can't reach.
            </h2>
            <p className="text-lg text-text-muted leading-relaxed mb-8">
              Claude Code token stats. Apple Podcasts history. Notes. Photos metadata. It's all there, locked on your machine. Meanwhile, people are buying Mac Minis to run AI agents — but even they can't easily feed local data into their automation stacks. You're left with fragile cron jobs that fail silently, or manual exports that break your flow.
            </p>

            {/* Problem bullets */}
            <ul className="space-y-3">
              {problems.map((problem, i) => (
                <li key={i} className="flex items-start gap-3">
                  <X className="w-5 h-5 text-problem flex-shrink-0 mt-0.5" />
                  <span className="text-text-muted">{problem}</span>
                </li>
              ))}
            </ul>
          </div>

          {/* RIGHT - Solution */}
          <div>
            <div className="text-success text-lg font-mono font-medium tracking-tight uppercase mb-4">
              The Solution
            </div>
            <h2 className="text-4xl font-bold mb-6 font-display">
              LocalPush unlocks it.
            </h2>
            <p className="text-lg text-text-muted leading-relaxed mb-8">
              A menu bar app that watches your local data and pushes changes to your automation server — n8n, Make, Zapier — or straight to a Google Sheet. Event-driven, not polling. Crash-safe, not 'fingers crossed.' You see your real data before anything is sent. Feed your AI agents with local data you control.
            </p>

            {/* Solution bullets */}
            <ul className="space-y-3">
              {solutions.map((solution, i) => (
                <li key={i} className="flex items-start gap-3">
                  <Check className="w-5 h-5 text-success flex-shrink-0 mt-0.5" />
                  <span className="text-text-muted">{solution}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </section>
  );
}
