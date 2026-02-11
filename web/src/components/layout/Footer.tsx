import { ArrowUpRight, Github } from "lucide-react";

export default function Footer() {
  return (
    <footer className="py-12 border-t border-border-muted">
      <div className="max-w-7xl mx-auto px-6 flex flex-col md:flex-row items-center justify-between gap-8">
        {/* Left: Logo + Brand */}
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 bg-primary rounded-md flex items-center justify-center">
            <ArrowUpRight className="w-4 h-4 text-bg-deep" />
          </div>
          <span className="text-base font-display font-semibold text-text-primary">
            LocalPush
          </span>
        </div>

        {/* Center: Attribution */}
        <p className="text-text-muted text-sm font-mono">
          Built by{" "}
          <a
            href="https://rightaim.ai"
            target="_blank"
            rel="noopener noreferrer"
            className="text-primary hover:text-primary/80 transition-colors"
          >
            Right Aim
          </a>{" "}
          · © 2026
        </p>

        {/* Right: GitHub Link */}
        <a
          href="https://github.com/madshn/localpush"
          target="_blank"
          rel="noopener noreferrer"
          className="flex items-center gap-2 text-text-muted hover:text-white font-mono text-sm transition-colors"
        >
          <Github className="w-4 h-4" />
          GitHub
        </a>
      </div>
    </footer>
  );
}
