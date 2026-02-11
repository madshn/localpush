import { ShieldCheck, CheckCircle2, Eye } from "lucide-react";

export default function TrustProof() {
  return (
    <section className="py-24">
      <div className="container mx-auto px-6">
        {/* Proof Badges */}
        <div className="flex flex-col md:flex-row justify-center items-center gap-12 mb-16">
          {/* Open Source Badge */}
          <div className="flex flex-col items-center text-center">
            <ShieldCheck className="w-12 h-12 text-success mb-3" />
            <div className="font-bold text-text-primary mb-1">
              Open Source (MIT)
            </div>
            <a
              href="https://github.com/madshn/localpush"
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm text-text-muted hover:text-primary transition-colors"
            >
              View on GitHub →
            </a>
          </div>

          {/* Guaranteed Delivery Badge */}
          <div className="flex flex-col items-center text-center">
            <CheckCircle2 className="w-12 h-12 text-primary mb-3" />
            <div className="font-bold text-text-primary mb-1">
              Guaranteed Delivery
            </div>
            <div className="text-sm text-text-muted">
              WAL-backed. Zero data loss.
            </div>
          </div>

          {/* Radical Transparency Badge */}
          <div className="flex flex-col items-center text-center">
            <Eye className="w-12 h-12 text-warning mb-3" />
            <div className="font-bold text-text-primary mb-1">
              Radical Transparency
            </div>
            <div className="text-sm text-text-muted">
              See your data before it's sent.
            </div>
          </div>
        </div>

        {/* Works With Section */}
        <div className="text-center">
          <h3 className="text-xs uppercase tracking-widest text-text-muted mb-6">
            Works With
          </h3>
          <div className="flex flex-wrap justify-center gap-4">
            <span className="px-3 py-1.5 bg-white/5 border border-border-muted rounded-md text-sm font-mono text-text-muted hover:text-white hover:border-primary/30 transition-colors">
              n8n
            </span>
            <span className="px-3 py-1.5 bg-white/5 border border-border-muted rounded-md text-sm font-mono text-text-muted hover:text-white hover:border-primary/30 transition-colors">
              Make
            </span>
            <span className="px-3 py-1.5 bg-white/5 border border-border-muted rounded-md text-sm font-mono text-text-muted hover:text-white hover:border-primary/30 transition-colors">
              Zapier
            </span>
            <span className="px-3 py-1.5 bg-white/5 border border-border-muted rounded-md text-sm font-mono text-text-muted hover:text-white hover:border-primary/30 transition-colors">
              Google Sheets
            </span>
            <span className="px-3 py-1.5 bg-white/5 border border-border-muted rounded-md text-sm font-mono text-text-muted hover:text-white hover:border-primary/30 transition-colors">
              ntfy
            </span>
          </div>
        </div>
      </div>
    </section>
  );
}
