import { useState } from "react";
import { useLocale } from "../i18n/LocaleProvider";
import { format } from "../i18n/locale";
import type { LibraryGame } from "../hooks/useLibrary";

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

interface Props {
  game: LibraryGame;
  onLaunch: () => void;
  onOpenFolder: () => void;
  onSetExe: () => void;
  onRefreshCover: () => void;
  onRemove: () => void;
}

/** One cover tile with its own actions menu; the page only passes callbacks. */
export function GameCard({
  game,
  onLaunch,
  onOpenFolder,
  onSetExe,
  onRefreshCover,
  onRemove,
}: Props) {
  const { t, locale } = useLocale();
  const [menuOpen, setMenuOpen] = useState(false);

  const pick = (action: () => void) => () => {
    setMenuOpen(false);
    action();
  };

  return (
    <li className="game-card">
      <button
        type="button"
        className="game-cover"
        onClick={onLaunch}
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
          aria-expanded={menuOpen}
          onClick={() => setMenuOpen(!menuOpen)}
        >
          ⋯
        </button>
        {menuOpen && (
          <ul className="game-menu" role="menu">
            <li>
              <button type="button" role="menuitem" onClick={pick(onOpenFolder)}>
                {t("library.openFolder")}
              </button>
            </li>
            <li>
              <button type="button" role="menuitem" onClick={pick(onSetExe)}>
                {t("library.setExe")}
              </button>
            </li>
            <li>
              <button type="button" role="menuitem" onClick={pick(onRefreshCover)}>
                {t("library.refreshCover")}
              </button>
            </li>
            <li>
              <button
                type="button"
                role="menuitem"
                className="game-menu-danger"
                onClick={pick(onRemove)}
              >
                {t("library.remove")}
              </button>
            </li>
          </ul>
        )}
      </div>
    </li>
  );
}
