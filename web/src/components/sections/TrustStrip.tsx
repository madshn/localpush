export default function TrustStrip() {
  const items = [
    'Open Source (MIT)',
    'Guaranteed Delivery',
    'See Your Data First',
    'Built for macOS',
  ];

  return (
    <section className="py-12 border-b border-border-muted">
      <div className="container mx-auto px-6">
        <div className="flex flex-wrap items-center justify-center gap-8 md:gap-16">
          {items.map((item, i) => (
            <div key={i} className="flex items-center gap-8 md:gap-16">
              <span
                className={`text-[10px] font-mono uppercase tracking-[0.2em] ${
                  item === 'Built for macOS'
                    ? 'text-primary font-bold'
                    : 'text-text-muted'
                }`}
              >
                {item}
              </span>
              {i < items.length - 1 && (
                <div className="w-1 h-1 bg-border-muted rounded-full hidden md:block" />
              )}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
