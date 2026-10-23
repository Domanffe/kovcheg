import { useLocale } from "../i18n/LocaleProvider";
import { cleanTitle } from "../util/titles";

export interface DownloadCandidate {
  id: string;
  name: string;
  dir: string;
}

interface Props {
  candidates: DownloadCandidate[];
  onAdd: (title: string, dir: string) => void;
  onInstall: (dir: string, title: string) => void;
  onClose: () => void;
}

/** "From downloads" sheet: folders that exist in history but are not in the library yet. */
export function LibraryAddSheet({ candidates, onAdd, onInstall, onClose }: Props) {
  const { t } = useLocale();

  return (
    <div className="sheet-backdrop" role="presentation" onClick={onClose}>
      <div
        className="sheet"
        role="dialog"
        aria-modal="true"
        aria-labelledby="library-sheet-title"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="sheet-header">
          <h2 id="library-sheet-title">{t("library.fromDownloads")}</h2>
          <button type="button" className="icon-btn" onClick={onClose}>
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
                        onClick={() => onAdd(title, item.dir)}
                      >
                        {t("library.addFolder")}
                      </button>
                      <button
                        type="button"
                        className="btn btn-secondary"
                        onClick={() => onInstall(item.dir, title)}
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
  );
}
