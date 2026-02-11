import { Link } from "react-router-dom";
import { ArrowLeft } from "lucide-react";
import Navbar from "@/components/layout/Navbar";
import Footer from "@/components/layout/Footer";

export default function BlogPage() {
  return (
    <div className="min-h-screen bg-bg-deep text-text-primary flex flex-col">
      <Navbar />

      <main className="flex-1 flex items-center justify-center px-6 py-24">
        <div className="max-w-2xl text-center">
          <h1 className="text-4xl md:text-5xl font-display font-bold mb-6">
            Blog coming soon
          </h1>
          <p className="text-lg text-text-muted mb-8 leading-relaxed">
            We're writing about Claude Code tracking, local AI agents, and
            unlocking Mac data.
          </p>
          <Link
            to="/"
            className="inline-flex items-center gap-2 text-primary hover:text-primary/80 font-mono text-sm transition-colors"
          >
            <ArrowLeft className="w-4 h-4" />
            Back to home
          </Link>
        </div>
      </main>

      <Footer />
    </div>
  );
}
