import { useState } from "react";

export type ModalStep = "closed" | "intent" | "auth" | "install";

export interface SignupModalState {
  isOpen: boolean;
  step: ModalStep;
  intent: string;
  customText: string;
  open: () => void;
  close: () => void;
  setStep: (step: ModalStep) => void;
  setIntent: (intent: string) => void;
  setCustomText: (text: string) => void;
  reset: () => void;
}

export function useSignupModal(): SignupModalState {
  const [isOpen, setIsOpen] = useState(false);
  const [step, setStep] = useState<ModalStep>("closed");
  const [intent, setIntent] = useState("");
  const [customText, setCustomText] = useState("");

  const open = () => {
    setIsOpen(true);
    setStep("intent");
  };

  const close = () => {
    setIsOpen(false);
    setStep("closed");
    setIntent("");
    setCustomText("");
  };

  const reset = () => {
    close();
  };

  return {
    isOpen,
    step,
    intent,
    customText,
    open,
    close,
    setStep,
    setIntent,
    setCustomText,
    reset,
  };
}

export default useSignupModal;
