import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { TorrentProvider, useTorrents } from "./hooks/useTorrents";
import { Shell } from "./components/Shell";
import { SplashView } from "./views/SplashView";
import { markSplashSeen, shouldSkipSplash } from "./util/splash";
import { isTauri } from "./util/tauri";

export type Section = "all" | "games" | "movies" | "tv" | "anime" | "library" | "downloads" | "seeding";

interface PendingLaunch {
  magnet: string;
  name: string;
  infoHash: string;
}

function AppInner({
  initialQuery,
  initialSection,
  pendingLaunch,
}: {
  initialQuery: string;
  initialSection: Section;
  pendingLaunch: PendingLaunch | null;
}) {
  const { addDownload } = useTorrents();
  const launched = useRef(false);

  useEffect(() => {
    if (!pendingLaunch || launched.current) return;
    launched.current = true;
    void addDownload({
      id: pendingLaunch.infoHash,
      name: pendingLaunch.name,
      magnet: pendingLaunch.magnet,
      source: "magnet",
      sizeBytes: 0,
    });
  }, [addDownload, pendingLaunch]);

  return <Shell initialQuery={initialQuery} initialSection={initialSection} />;
}

export function App() {
  const [boot, setBoot] = useState<{
    pending: PendingLaunch | null;
    showSplash: boolean;
  } | null>(null);
  const [initialQuery, setInitialQuery] = useState("");

  useEffect(() => {
    if (!isTauri()) {
      setBoot({ pending: null, showSplash: !shouldSkipSplash() });
      return;
    }
    void invoke<PendingLaunch | null>("take_pending_launch").then((pending) => {
      if (pending) markSplashSeen();
      setBoot({ pending, showSplash: pending ? false : !shouldSkipSplash() });
    });
  }, []);

  const enterApp = useCallback((query: string): void => {
    markSplashSeen();
    setInitialQuery(query);
    setBoot((b) => (b ? { ...b, showSplash: false } : b));
  }, []);

  if (!boot) return null;

  if (boot.showSplash) {
    return (
      <SplashView
        onBrowse={() => enterApp("")}
        onSearch={(q: string) => enterApp(q)}
      />
    );
  }

  return (
    <TorrentProvider>
      <AppInner
        initialQuery={initialQuery}
        initialSection={boot.pending ? "downloads" : "all"}
        pendingLaunch={boot.pending}
      />
    </TorrentProvider>
  );
}
