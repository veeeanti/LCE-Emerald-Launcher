import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { TauriService, PlaytimeResponse } from "../../services/TauriService";

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}h ${m}m ${s}s`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

export default function PlaytimeModal({
  isOpen,
  onClose,
  instanceId,
  instanceName,
  playBackSound,
}: {
  isOpen: boolean;
  onClose: () => void;
  instanceId: string;
  instanceName: string;
  playBackSound: (s?: string) => void;
}) {
  const [playtime, setPlaytime] = useState<PlaytimeResponse | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!isOpen) {
      setPlaytime(null);
      return;
    }
    setLoading(true);
    TauriService.getPlaytime(instanceId)
      .then(setPlaytime)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [isOpen, instanceId]);

  useEffect(() => {
    if (!isOpen) return;
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        playBackSound("close_click.wav");
        onClose();
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [isOpen, onClose, playBackSound]);

  if (!isOpen) return null;

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 w-screen h-screen z-[100] flex items-center justify-center bg-black/80 backdrop-blur-md outline-none border-none"
    >
      <div
        className="relative w-[380px] p-6 flex flex-col items-center shadow-2xl"
        style={{
          backgroundImage: "url('/images/frame_background.png')",
          backgroundSize: "100% 100%",
          imageRendering: "pixelated",
        }}
      >
        <h2 className="text-[#FFFF55] text-2xl mc-text-shadow mb-4 border-b-2 border-[#373737] pb-2 w-full text-center uppercase">
          Playtime
        </h2>

        <p className="text-white text-sm mc-text-shadow mb-5 text-center">
          {instanceName}
        </p>

        {loading ? (
          <div className="text-gray-400 text-sm mc-text-shadow mb-4">Loading...</div>
        ) : playtime ? (
          <div className="w-full flex flex-col gap-3 mb-4">
            <div className="flex justify-between items-center bg-black/30 px-4 py-3 border border-[#373737]">
              <span className="text-[#AAAAAA] text-sm mc-text-shadow uppercase tracking-wider">
                Total
              </span>
              <span className="text-white text-lg mc-text-shadow font-bold">
                {formatTime(playtime.totalSeconds)}
              </span>
            </div>
            <div className="flex justify-between items-center bg-black/30 px-4 py-3 border border-[#373737]">
              <span className="text-[#AAAAAA] text-sm mc-text-shadow uppercase tracking-wider">
                This Week
              </span>
              <span className="text-white text-lg mc-text-shadow font-bold">
                {formatTime(playtime.weekSeconds)}
              </span>
            </div>
            <div className="flex justify-between items-center bg-black/30 px-4 py-3 border border-[#373737]">
              <span className="text-[#AAAAAA] text-sm mc-text-shadow uppercase tracking-wider">
                Today
              </span>
              <span className="text-white text-lg mc-text-shadow font-bold">
                {formatTime(playtime.daySeconds)}
              </span>
            </div>
          </div>
        ) : (
          <div className="text-red-400 text-sm mc-text-shadow mb-4">Failed to load playtime data.</div>
        )}

        <button
          onClick={() => {
            playBackSound("close_click.wav");
            onClose();
          }}
          className="w-32 h-10 flex items-center justify-center text-xl mc-text-shadow transition-colors outline-none border-none text-white"
          style={{
            backgroundImage: "url('/images/button_highlighted.png')",
            backgroundSize: "100% 100%",
            imageRendering: "pixelated",
          }}
        >
          Close
        </button>
      </div>
    </motion.div>
  );
}
