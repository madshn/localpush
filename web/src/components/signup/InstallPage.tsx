import { useState } from "react";
import { CheckCircle2, Copy, Check, Download } from "lucide-react";

interface InstallPageProps {
  onClose: () => void;
}

export default function InstallPage(_props: InstallPageProps) {
  const [copied, setCopied] = useState(false);

  const brewCommand = "brew tap madshn/localpush && brew install --cask localpush";

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(brewCommand);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  };

  return (
    <div className="text-center">
      <div className="flex justify-center mb-6">
        <CheckCircle2 className="w-16 h-16 text-success" />
      </div>

      <h2 className="text-3xl font-bold mb-2">You're in.</h2>
      <p className="text-text-muted mb-8">
        Ready to unlock your Mac's data power.
      </p>

      <div className="bg-black rounded-lg border border-white/10 overflow-hidden mb-6">
        {/* Fake terminal chrome */}
        <div className="flex items-center gap-2 px-4 py-2 bg-bg-elevated border-b border-white/10">
          <div className="flex gap-1.5">
            <div className="w-3 h-3 rounded-full bg-red-500/50" />
            <div className="w-3 h-3 rounded-full bg-amber-500/50" />
            <div className="w-3 h-3 rounded-full bg-green-500/50" />
          </div>
          <span className="text-xs text-text-muted ml-2">Terminal</span>
        </div>

        {/* Command area */}
        <div className="relative p-4 font-mono text-sm">
          <div className="flex items-start gap-2">
            <span className="text-primary flex-shrink-0">$</span>
            <span className="text-white flex-1 break-all">{brewCommand}</span>
          </div>

          <button
            onClick={handleCopy}
            className="absolute top-2 right-2 p-2 bg-bg-elevated hover:bg-white/10 rounded border border-white/10 transition-colors"
            aria-label="Copy command"
          >
            {copied ? (
              <Check className="w-4 h-4 text-success" />
            ) : (
              <Copy className="w-4 h-4 text-text-muted" />
            )}
          </button>
        </div>
      </div>

      <a
        href="#"
        className="w-full inline-flex items-center justify-center gap-2 py-3 bg-primary text-bg-deep font-bold rounded-lg hover:bg-primary/90 transition-all mb-4"
      >
        <Download className="w-5 h-5" />
        Download macOS App
      </a>

      <div className="flex items-center justify-center gap-6 text-sm font-medium text-text-muted">
        <a
          href="https://github.com/madshn/localpush"
          target="_blank"
          rel="noopener noreferrer"
          className="hover:text-primary transition-colors"
        >
          Star on GitHub →
        </a>
        <a
          href="https://discord.gg/localpush"
          target="_blank"
          rel="noopener noreferrer"
          className="hover:text-primary transition-colors"
        >
          Join Discord →
        </a>
      </div>
    </div>
  );
}
