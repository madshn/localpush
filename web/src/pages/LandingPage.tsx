import Navbar from "@/components/layout/Navbar";
import Hero from "@/components/sections/Hero";
import TrustStrip from "@/components/sections/TrustStrip";
import ProblemSolution from "@/components/sections/ProblemSolution";
import HowItWorks from "@/components/sections/HowItWorks";
import UseCases from "@/components/sections/UseCases";
import DidYouKnow from "@/components/sections/DidYouKnow";
import TrustProof from "@/components/sections/TrustProof";
import EarlyAccessCTA from "@/components/sections/EarlyAccessCTA";
import BlogPreview from "@/components/sections/BlogPreview";
import Footer from "@/components/layout/Footer";
import SignupModal from "@/components/signup/SignupModal";
import { useSignupModal } from "@/hooks/use-signup-modal";

export default function LandingPage() {
  const {
    isOpen,
    step,
    intent,
    customText,
    open,
    close,
    setStep,
    setIntent,
    setCustomText,
  } = useSignupModal();

  return (
    <div className="min-h-screen bg-bg-deep text-text-primary">
      <Navbar />

      <main>
        <Hero onOpenSignup={open} />
        <TrustStrip />
        <ProblemSolution />
        <HowItWorks />
        <UseCases />
        <DidYouKnow />
        <TrustProof />
        <EarlyAccessCTA onOpenSignup={open} />
        <BlogPreview />
      </main>

      <Footer />

      <SignupModal
        isOpen={isOpen}
        step={step}
        intent={intent}
        customText={customText}
        close={close}
        setStep={setStep}
        setIntent={setIntent}
        setCustomText={setCustomText}
      />
    </div>
  );
}
