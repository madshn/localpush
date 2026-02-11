import { X } from "lucide-react";
import { type ModalStep } from "@/hooks/use-signup-modal";
import IntentCapture from "@/components/signup/IntentCapture";
import SocialAuth from "@/components/signup/SocialAuth";
import InstallPage from "@/components/signup/InstallPage";

interface SignupModalProps {
  isOpen: boolean;
  step: ModalStep;
  intent: string;
  customText: string;
  close: () => void;
  setStep: (step: ModalStep) => void;
  setIntent: (intent: string) => void;
  setCustomText: (text: string) => void;
}

export default function SignupModal({
  isOpen,
  step,
  intent,
  customText,
  close,
  setStep,
  setIntent,
  setCustomText,
}: SignupModalProps) {
  if (!isOpen) return null;

  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) {
      close();
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm"
      onClick={handleBackdropClick}
    >
      <div className="relative w-full max-w-lg bg-bg-surface rounded-xl shadow-2xl border border-white/10 overflow-hidden">
        <button
          onClick={close}
          className="absolute top-4 right-4 z-10 p-2 text-text-muted hover:text-text-primary transition-colors"
          aria-label="Close modal"
        >
          <X className="w-5 h-5" />
        </button>

        <div className="p-8">
          {step === "intent" && (
            <IntentCapture
              intent={intent}
              customText={customText}
              setIntent={setIntent}
              setCustomText={setCustomText}
              onContinue={() => setStep("auth")}
            />
          )}

          {step === "auth" && (
            <SocialAuth onSuccess={() => setStep("install")} />
          )}

          {step === "install" && (
            <InstallPage onClose={close} />
          )}
        </div>
      </div>
    </div>
  );
}
