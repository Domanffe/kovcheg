import type { Section } from "../App";
import { useLocale } from "../i18n/LocaleProvider";
import {
  DownloadIcon,
  GamepadIcon,
  GlobeIcon,
  LibraryIcon,
  PlayerIcon,
  RadioIcon,
  SparklesIcon,
  UiIcon,
  UploadIcon,
  type IconComponent,
} from "../icons";

const FILTERS: { key: Section; labelKey: string; icon: IconComponent }[] = [
  { key: "all", labelKey: "section.all", icon: GlobeIcon },
  { key: "games", labelKey: "section.games", icon: GamepadIcon },
  { key: "movies", labelKey: "section.movies", icon: PlayerIcon },
  { key: "tv", labelKey: "section.tv", icon: RadioIcon },
  { key: "anime", labelKey: "section.anime", icon: SparklesIcon },
];

const LIBRARY: { key: Section; labelKey: string; icon: IconComponent }[] = [
  { key: "library", labelKey: "section.library", icon: LibraryIcon },
  { key: "downloads", labelKey: "section.downloads", icon: DownloadIcon },
  { key: "seeding", labelKey: "section.seeding", icon: UploadIcon },
];

export function Sidebar({
  section,
  onSection,
  downloadCount,
  seedCount,
}: {
  section: Section;
  onSection: (s: Section) => void;
  downloadCount: number;
  seedCount: number;
}) {
  const { t } = useLocale();

  const badge = (key: Section): number | null => {
    if (key === "downloads" && downloadCount > 0) return downloadCount;
    if (key === "seeding" && seedCount > 0) return seedCount;
    return null;
  };

  return (
    <nav className="sidebar" aria-label={t("nav.aria")}>
      <span className="nav-label">{t("nav.browse")}</span>
      <ul className="nav-group">
        {FILTERS.map((item) => (
          <li key={item.key}>
            <button
              type="button"
              className={`nav-item${section === item.key ? " active" : ""}`}
              onClick={() => onSection(item.key)}
            >
              <span className="nav-icon" aria-hidden>
                <UiIcon icon={item.icon} size={17} />
              </span>
              {t(item.labelKey)}
            </button>
          </li>
        ))}
      </ul>
      <span className="nav-label">{t("nav.library")}</span>
      <ul className="nav-group">
        {LIBRARY.map((item) => {
          const n = badge(item.key);
          return (
            <li key={item.key}>
              <button
                type="button"
                className={`nav-item${section === item.key ? " active" : ""}`}
                onClick={() => onSection(item.key)}
              >
                <span className="nav-icon" aria-hidden>
                  <UiIcon icon={item.icon} size={17} />
                </span>
                {t(item.labelKey)}
                {n != null && <span className="nav-badge">{n}</span>}
              </button>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
