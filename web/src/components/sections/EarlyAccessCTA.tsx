interface EarlyAccessCTAProps {
  onOpenSignup: () => void;
}

export default function EarlyAccessCTA({ onOpenSignup }: EarlyAccessCTAProps) {
  return (
    <section className="py-24 bg-primary/5">
      <div className="container mx-auto px-6">
        <div className="max-w-3xl mx-auto text-center">
          <h2 className="text-4xl font-bold tracking-tight mb-6">
            Join the beta.
          </h2>
          <p className="text-lg text-text-muted mb-8">
            LocalPush is in testing and launches as open source soon. Sign up
            now to get immediate access and help shape what ships.
          </p>

          <button
            onClick={onOpenSignup}
            className="px-12 py-4 bg-primary text-bg-deep font-bold rounded-lg hover:bg-primary/90 transition-all shadow-xl shadow-primary/20"
          >
            Become an Early Tester
          </button>

          <p className="mt-8 text-sm text-text-muted font-mono">
            Prefer to wait for the open source launch?{" "}
            <a
              href="https://github.com/madshn/localpush"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-primary transition-colors"
            >
              Follow on GitHub →
            </a>
          </p>
        </div>
      </div>
    </section>
  );
}
