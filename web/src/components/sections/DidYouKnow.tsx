import { Link } from "react-router-dom";

export default function DidYouKnow() {
  return (
    <section className="py-24 border-y border-border-muted overflow-hidden">
      <div className="container mx-auto px-6">
        <h2 className="text-sm uppercase tracking-widest text-text-muted font-bold text-center mb-16">
          Did you know?
        </h2>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6 mb-6">
          {/* Card 1 - Apple Podcasts */}
          <div className="p-6 rounded-lg border-l-4 border-success bg-white/[0.02]">
            <div className="font-bold text-success mb-3">Apple Podcasts</div>
            <p className="text-sm text-text-muted leading-relaxed mb-4">
              Apple Podcasts stores your complete listening history — every
              episode, when you played it, how far you got — and full
              transcripts of everything you listened to. It's all sitting in a
              SQLite database on your Mac right now.
            </p>
            <div className="text-primary font-bold">LocalPush it.</div>
          </div>

          {/* Card 2 - Claude Code */}
          <div className="p-6 rounded-lg border-l-4 border-primary bg-white/[0.02]">
            <div className="font-bold text-primary mb-3">Claude Code</div>
            <p className="text-sm text-text-muted leading-relaxed mb-4">
              Claude Code stores every session name, the project you worked on,
              which git branch, timestamps, message counts, and your daily token
              spend broken down by model. Your entire AI work history lives in{" "}
              <code className="bg-white/5 px-1.5 py-0.5 rounded text-xs font-mono">
                ~/.claude/
              </code>
              .
            </p>
            <div className="text-primary font-bold">LocalPush it.</div>
          </div>

          {/* Card 3 - Apple Notes */}
          <div className="p-6 rounded-lg border-l-4 border-warning bg-white/[0.02]">
            <div className="font-bold text-warning mb-3">Apple Notes</div>
            <p className="text-sm text-text-muted leading-relaxed mb-4">
              Every note you've written — when you created it, when you last
              touched it, which folder it's in — is tracked in a database on
              your Mac. Your note-taking patterns tell a story.
            </p>
            <div className="text-primary font-bold">LocalPush it.</div>
          </div>

          {/* Card 4 - Apple Photos */}
          <div className="p-6 rounded-lg border-l-4 border-pink-500 bg-white/[0.02]">
            <div className="font-bold text-pink-500 mb-3">Apple Photos</div>
            <p className="text-sm text-text-muted leading-relaxed mb-4">
              Your Photos library tracks metadata on every image — when, where,
              what camera, what's in the photo. Thousands of data points about
              your visual life, sitting in a SQLite database right now.
            </p>
            <div className="text-primary font-bold">LocalPush it.</div>
          </div>
        </div>

        {/* Card 5 - What's Next */}
        <div className="max-w-lg mx-auto p-6 rounded-lg border border-dashed border-white/30 bg-white/[0.02]">
          <div className="font-bold text-white mb-3">What's Next?</div>
          <p className="text-sm text-text-muted leading-relaxed mb-4">
            Your Mac knows your browsing history, screen time, calendar
            patterns, music taste, and more. We're unlocking new sources every
            release.
          </p>
          <Link
            to="https://discord.gg/localpush"
            className="text-primary font-bold hover:underline"
          >
            What would YOU LocalPush?
          </Link>
        </div>

        <p className="text-xl text-center mt-12 text-text-primary">
          Don't let it sit there. <strong>LocalPush it.</strong>
        </p>
      </div>
    </section>
  );
}
