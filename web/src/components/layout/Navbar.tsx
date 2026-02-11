import { ArrowUpRight, Star } from "lucide-react";

export default function Navbar() {
  return (
    <nav className="sticky top-0 z-50 border-b border-border-muted bg-bg-deep/80 backdrop-blur-md">
      <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
        {/* Left: Logo + Brand */}
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 bg-primary rounded-md flex items-center justify-center">
            <ArrowUpRight className="w-5 h-5 text-bg-deep" />
          </div>
          <span className="text-lg font-display font-semibold text-text-primary">
            LocalPush
          </span>
          <span className="px-2 py-0.5 bg-primary/10 text-primary text-[10px] font-mono font-bold border border-primary/20 rounded-full tracking-wider uppercase">
            Beta
          </span>
        </div>

        {/* Right: GitHub Link */}
        <a
          href="https://github.com/madshn/localpush"
          target="_blank"
          rel="noopener noreferrer"
          className="flex items-center gap-2 text-text-muted hover:text-white font-mono text-sm transition-colors"
        >
          <Star className="w-4 h-4" />
          Star on GitHub →
        </a>
      </div>
    </nav>
  );
}
