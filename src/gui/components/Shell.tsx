import { lazy, Suspense, useEffect, useState } from "react";
import type { Section } from "../App";
import { SettingsSheet } from "../components/SettingsSheet";
import { Sidebar } from "../components/Sidebar";
import { Wordmark } from "../components/Wordmark";
import { runUpdateCheck } from "../util/updateCheck";
import { useTorrents } from "../hooks/useTorrents";
import { useLocale } from "../i18n/LocaleProvider";
import { GearIcon, UiIcon } from "../icons";
import { SECTION_ORDER, SEARCH_SECTIONS } from "../sections";

const SearchView = lazy(() => import("../views/SearchView").then((m) => ({ default: m.SearchView })));
const DownloadsView = lazy(() =>
  import("../views/DownloadsView").then((m) => ({ default: m.DownloadsView })),
);
const SeedingView = lazy(() =>
  import("../views/SeedingView").then((m) => ({ default: m.SeedingView })),
);
const LibraryView = lazy(() =>
  import("../views/LibraryView").then((m) => ({ default: m.LibraryView })),
);

export function Shell({
  initialQuery = "",
  initialSection = "all" as Section,
}: {
  initialQuery?: string;
  initialSection?: Section;
}) {
  const { t } = useLocale();
  const [section, setSection] = useState<Section>(initialSection);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const { list, config, refreshConfig, tauri } = useTorrents();
  const activeDl = list.downloads.filter((d) => d.status === "downloading").length;
  const activeSeed = list.seeds.filter((s) => s.status === "seeding").length;

  useEffect(() => {
    if (!tauri) return;
    const timer = window.setTimeout(() => void runUpdateCheck(false), 4000);
    return () => window.clearTimeout(timer);
  }, [tauri]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "," && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        setSettingsOpen(true);
      }
      if (e.key === "Tab" && !e.ctrlKey && !e.metaKey) {
        const tag = document.activeElement?.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA") return;
        e.preventDefault();
        const i = SECTION_ORDER.indexOf(section);
        setSection(SECTION_ORDER[(i + 1) % SECTION_ORDER.length]!);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [section]);

  return (
    <div className="shell">
      <header className="header">
        <Wordmark size="sm" />
        <div className="header-actions">
          <button
            type="button"
            className="icon-btn"
            onClick={() => setSettingsOpen(true)}
            title={t("shell.settingsTitle")}
            aria-label={t("shell.settings")}
          >
            <UiIcon icon={GearIcon} size={20} />
          </button>
        </div>
      </header>
      <div className="body">
        <Sidebar
          section={section}
          onSection={setSection}
          downloadCount={activeDl}
          seedCount={activeSeed}
        />
        <main className="main">
          <Suspense fallback={<p className="status">{t("shell.loading")}</p>}>
            {SEARCH_SECTIONS.has(section) && (
              <SearchView category={section} initialQuery={initialQuery} />
            )}
            {section === "library" && <LibraryView />}
            {section === "downloads" && <DownloadsView />}
            {section === "seeding" && <SeedingView />}
          </Suspense>
        </main>
      </div>

      <SettingsSheet
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        config={config}
        onSaved={() => void refreshConfig()}
      />
    </div>
  );
}
