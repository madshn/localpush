import { Link } from "react-router-dom";
import { ArrowRight } from "lucide-react";

export default function BlogPreview() {
  const posts = [
    {
      title: "Track Your Claude Code Token Spend in 5 Minutes",
      summary: "Set up automatic AI spend tracking with LocalPush and n8n.",
      readTime: "4 min read",
    },
    {
      title: "Unlock Your Mac's Hidden Data",
      summary:
        "Discover what Apple Podcasts, Notes, and Photos store about you.",
      readTime: "6 min read",
    },
    {
      title: "Beyond OpenClaw: Building a Local AI Agent Team",
      summary: "Feed your self-hosted AI agents with local data you control.",
      readTime: "8 min read",
    },
  ];

  return (
    <section className="py-24">
      <div className="container mx-auto px-6">
        <h2 className="text-sm uppercase tracking-widest text-text-muted font-bold text-center mb-16">
          From the Blog
        </h2>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {posts.map((post, index) => (
            <Link
              key={index}
              to="/blog"
              className="p-6 rounded-xl bg-bg-surface border border-border-muted hover:border-primary/30 transition-colors"
            >
              <h3 className="text-lg font-bold mb-2">{post.title}</h3>
              <p className="text-sm text-text-muted mb-4">{post.summary}</p>
              <div className="flex items-center gap-2 text-xs text-primary font-mono">
                <span>{post.readTime}</span>
                <ArrowRight className="w-3 h-3" />
              </div>
            </Link>
          ))}
        </div>
      </div>
    </section>
  );
}
