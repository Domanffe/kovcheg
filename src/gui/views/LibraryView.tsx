import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Card } from "../components/Card";
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

function initials(title: string): string {
  return title
    .split(/\s+/)
    .filter((w) => /[a-zа-я0-9]/i.test(w))
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase() ?? "")
    .join("");
}

function playtimeText(secs: number, locale: string): string {
  const hours = Math.floor(secs / 3600);
  const minutes = Math.round((secs % 3600) / 60);
  if (hours === 0) {
    if (minutes === 0) return locale === "ru" ? "<1 мин" : "<1 min";
    return locale === "ru" ? `${minutes} мин` : `${minutes} min`;
  }
  return locale === "ru" ? `${hours} ч ${minutes} мин` : `${hours} h ${minutes} min`;
}

export function LibraryView() {
  const { t, locale, messages } = useLocale();
  const { toast } = useToast();
  const { list } = useTorrents();
  const lib = useLibrary();
  const [menuOpen, setMenuOpen] = useState<string | null>(null);
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
            <li key={game.id} className="game-card">
              <button
                type="button"
                className="game-cover"
                onClick={() => void lib.launch(game.id)}
                disabled={game.running}
                aria-label={format(t("library.play"), { name: game.title })}
              >
                {game.cover ? (
                  <img className="game-cover-img" src={game.cover} alt="" loading="lazy" />
                ) : (
                  <span className="game-cover-fallback" aria-hidden>
                    {initials(game.title)}
                  </span>
                )}
                <span className="game-play" aria-hidden>
                  ▶
                </span>
                {game.running && <span className="game-badge">{t("library.playingNow")}</span>}
              </button>
              <div className="game-name" title={game.title}>
                {game.title}
              </div>
              <div className="game-sub">
                {game.running
                  ? t("library.playingNow")
                  : game.playtimeSecs > 0
                    ? playtimeText(game.playtimeSecs, locale)
                    : t("library.neverPlayed")}
              </div>
              <div className="game-menu-wrap">
                <button
                  type="button"
                  className="icon-btn game-more"
                  aria-label={t("library.actions")}
                  aria-expanded={menuOpen === game.id}
                  onClick={() => setMenuOpen(menuOpen === game.id ? null : game.id)}
                >
                  ⋯
                </button>
                {menuOpen === game.id && (
                  <ul className="game-menu" role="menu">
                    <li>
                      <button
                        type="button"
                        role="menuitem"
                        onClick={() => {
                          setMenuOpen(null);
                          void lib.openFolder(game.id);
                        }}
                      >
                        {t("library.openFolder")}
                      </button>
                    </li>
                    <li>
                      <button
                        type="button"
                        role="menuitem"
                        onClick={() => {
                          setMenuOpen(null);
                          void pickExe(game);
                        }}
                      >
                        {t("library.setExe")}
                      </button>
                    </li>
                    <li>
                      <button
                        type="button"
                        role="menuitem"
                        onClick={() => {
                          setMenuOpen(null);
                          void lib.refreshCover(game.id);
                        }}
                      >
                        {t("library.refreshCover")}
                      </button>
                    </li>
                    <li>
                      <button
                        type="button"
                        role="menuitem"
                        className="game-menu-danger"
                        onClick={() => {
                          setMenuOpen(null);
                          void removeGame(game);
                        }}
                      >
                        {t("library.remove")}
                      </button>
                    </li>
                  </ul>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}

      {sheetOpen && (
        <div className="sheet-backdrop" role="presentation" onClick={() => setSheetOpen(false)}>
          <div
            className="sheet"
            role="dialog"
            aria-modal="true"
            aria-labelledby="library-sheet-title"
            onClick={(e) => e.stopPropagation()}
          >
            <header className="sheet-header">
              <h2 id="library-sheet-title">{t("library.fromDownloads")}</h2>
              <button type="button" className="icon-btn" onClick={() => setSheetOpen(false)}>
                {t("library.close")}
              </button>
            </header>
            <div className="sheet-body">
              <p className="field-hint">{t("library.fromDownloadsHint")}</p>
              {candidates.length === 0 ? (
                <p className="empty">{t("library.fromDownloadsEmpty")}</p>
              ) : (
                <ul className="download-list">
                  {candidates.map((item) => {
                    const title = cleanTitle(item.name);
                    return (
                      <li key={item.id} className="download-item">
                        <div className="download-head">
                          <span className="download-name">{item.name}</span>
                        </div>
                        <div className="download-actions">
                          <button
                            type="button"
                            className="btn btn-ghost"
                            onClick={() => {
                              setSheetOpen(false);
                              void lib.add(title, item.dir);
                            }}
                          >
                            {t("library.addFolder")}
                          </button>
                          <button
                            type="button"
                            className="btn btn-secondary"
                            onClick={() => {
                              setSheetOpen(false);
                              void lib.install(item.dir, title);
                            }}
                          >
                            {t("library.install")}
                          </button>
                        </div>
                      </li>
                    );
                  })}
                </ul>
              )}
            </div>
          </div>
        </div>
      )}
    </Card>
  );
}
