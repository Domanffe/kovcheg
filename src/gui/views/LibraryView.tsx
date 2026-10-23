import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Card } from "../components/Card";
import { GameCard } from "../components/GameCard";
import { LibraryAddSheet } from "../components/LibraryAddSheet";
import { useToast } from "../components/Toast";
import { useLibrary, type LibraryGame } from "../hooks/useLibrary";
import { useTorrents } from "../hooks/useTorrents";
import { useLocale } from "../i18n/LocaleProvider";
import { format } from "../i18n/locale";
import { LibraryIcon, UiIcon } from "../icons";
import { confirmRemoveGame } from "../util/confirm";
import { errMsg } from "../util/errors";
import { cleanTitle } from "../util/titles";

function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function LibraryView() {
  const { t, messages } = useLocale();
  const { toast } = useToast();
  const { list } = useTorrents();
  const lib = useLibrary();
  const [sheetOpen, setSheetOpen] = useState(false);

  const knownDirs = new Set(lib.games.map((g) => g.dir.toLowerCase()));
  const candidates = list.history.filter((item) => !knownDirs.has(item.dir.toLowerCase()));

  const addFromFolder = async (): Promise<void> => {
    try {
      const picked = await open({ directory: true, multiple: false, title: t("library.pickFolderTitle") });
      if (typeof picked !== "string") return;
      await lib.add(cleanTitle(basename(picked)), picked);
      toast(t("library.added"), "success");
    } catch (e) {
      toast(errMsg(e), "error");
    }
  };

  const importFolder = async (): Promise<void> => {
    try {
      const picked = await open({
        directory: true,
        multiple: false,
        title: t("library.pickImportTitle"),
      });
      if (typeof picked !== "string") return;
      const report = await lib.importFolder(picked);
      if (!report) return;
      toast(format(t("library.imported"), { count: report.added.length }), "success");
      if (report.skipped.length > 0) {
        toast(
          format(t("library.importSkipped"), { names: report.skipped.slice(0, 5).join(", ") }),
          "info",
        );
      }
    } catch (e) {
      toast(errMsg(e), "error");
    }
  };

  const pickExe = async (game: LibraryGame): Promise<void> => {
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        title: t("library.pickExeTitle"),
        filters: [{ name: "exe", extensions: ["exe"] }],
      });
      if (typeof picked !== "string") return;
      await lib.setExe(game.id, picked);
    } catch (e) {
      toast(errMsg(e), "error");
    }
  };

  const removeGame = async (game: LibraryGame): Promise<void> => {
    const choice = await confirmRemoveGame(game.title, messages.confirm);
    if (choice === "keep") await lib.remove(game.id, false);
    else if (choice === "delete") await lib.remove(game.id, true);
  };

  return (
    <Card
      title={t("library.title")}
      subtitle={lib.games.length ? t("library.count", { count: lib.games.length }) : undefined}
    >
      <div className="library-toolbar">
        <button type="button" className="btn btn-primary" onClick={() => void addFromFolder()}>
          {t("library.addGame")}
        </button>
        <button type="button" className="btn btn-secondary" onClick={() => setSheetOpen(true)}>
          {t("library.fromDownloads")}
        </button>
        <button type="button" className="btn btn-secondary" onClick={() => void importFolder()}>
          {t("library.import")}
        </button>
      </div>

      {lib.installing && (
        <p className="install-banner" role="status">
          <strong>{lib.installing.title}</strong> — {lib.installing.message}
        </p>
      )}

      {lib.games.length === 0 ? (
        <p className="empty">
          <span className="empty-icon" aria-hidden>
            <UiIcon icon={LibraryIcon} size={26} />
          </span>
          {t("library.empty")}
          <span className="empty-hint">{t("library.emptyHint")}</span>
        </p>
      ) : (
        <ul className="library-grid">
          {lib.games.map((game) => (
            <GameCard
              key={game.id}
              game={game}
              onLaunch={() => void lib.launch(game.id)}
              onOpenFolder={() => void lib.openFolder(game.id)}
              onSetExe={() => void pickExe(game)}
              onRefreshCover={() => void lib.refreshCover(game.id)}
              onRemove={() => void removeGame(game)}
            />
          ))}
        </ul>
      )}

      {sheetOpen && (
        <LibraryAddSheet
          candidates={candidates}
          onAdd={(title, dir) => {
            setSheetOpen(false);
            void lib.add(title, dir);
          }}
          onInstall={(dir, title) => {
            setSheetOpen(false);
            void lib.install(dir, title);
          }}
          onClose={() => setSheetOpen(false)}
        />
      )}
    </Card>
  );
}
