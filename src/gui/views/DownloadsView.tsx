import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card } from "../components/Card";
import { useToast } from "../components/Toast";
import { TorrentProgress } from "../components/TorrentProgress";
import { truncate } from "../../util/format";
import { sourceStyle } from "../../sources/style";
import type { HistoryItem, QueueItem } from "../hooks/useTorrents";
import { useTorrents } from "../hooks/useTorrents";
import { useFormat } from "../hooks/useFormat";
import { useLocale } from "../i18n/LocaleProvider";
import { format } from "../i18n/locale";
import {
  BatteryPauseIcon,
  DownloadIcon,
  TriangleAlertIcon,
  UiIcon,
} from "../icons";
import { confirmCancelDownload, confirmClearHistory, confirmRemoveHistory } from "../util/confirm";
import { useDesktopActions } from "../hooks/useDesktopActions";
import { errMsg } from "../util/errors";
import { cleanTitle } from "../util/titles";

function isFailed(it: QueueItem): boolean {
  return it.status === "failed";
}

function isPaused(it: QueueItem): boolean {
  return it.status === "paused";
}

function folderName(dir: string): string {
  const normalized = dir.replace(/\\/g, "/").replace(/\/+$/, "");
  const idx = normalized.lastIndexOf("/");
  return idx >= 0 ? normalized.slice(idx + 1) : normalized;
}

export function DownloadsView() {
  const { t, messages } = useLocale();
  const { revealFolder } = useDesktopActions();
  const fmt = useFormat();
  const { list, addDownload, pause, resume, retry, remove, removeHistory, clearHistory } =
    useTorrents();
  const items = list.downloads;
  const history = list.history;
  const confirm = messages.confirm;
  const { toast } = useToast();
  const [installingId, setInstallingId] = useState<string | null>(null);

  const installRepack = async (h: HistoryItem): Promise<void> => {
    setInstallingId(h.id);
    try {
      const title = cleanTitle(h.name);
      await invoke("library_install", { dir: h.dir, title, installDir: null });
      toast(format(t("library.installed"), { name: title }), "success");
    } catch (e) {
      toast(errMsg(e), "error");
    } finally {
      setInstallingId(null);
    }
  };

  const cancelItem = async (it: QueueItem): Promise<void> => {
    const choice = await confirmCancelDownload(truncate(it.name, 40), confirm);
    if (choice === "keep") await remove(it.id, false);
    else if (choice === "delete") await remove(it.id, true);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const it = items[0];
      if (!it) return;
      if (e.key === "p") {
        if (it.status === "downloading") void pause(it.id);
        else if (it.status === "paused") void resume(it.id);
      }
      if (e.key === "f" && isFailed(it)) void retry(it.id);
      if (e.key === "c") void cancelItem(it);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [items, pause, resume, retry, remove]);

  const redownload = (h: HistoryItem): void => {
    void addDownload({
      id: h.id,
      name: h.name,
      magnet: h.magnet,
      source: h.source,
      sizeBytes: h.sizeBytes,
    });
  };

  const clearAll = async (): Promise<void> => {
    if (await confirmClearHistory(confirm)) await clearHistory();
  };

  return (
    <div className="downloads-view">
      <Card
        title={t("downloads.active")}
        subtitle={items.length ? t("downloads.items", { count: items.length }) : undefined}
      >
        {items.length === 0 ? (
          <p className="empty">{t("downloads.empty")}</p>
        ) : (
          <ul className="download-list">
            {items.map((it) => {
              const paused = isPaused(it);
              const failed = isFailed(it);
              const ss = it.source ? sourceStyle(it.source) : null;
              return (
                <li key={it.id} className={`download-item${failed ? " download-item--failed" : ""}`}>
                  <div className="download-head">
                    <span className="download-icon" aria-hidden>
                      {failed ? (
                        <UiIcon icon={TriangleAlertIcon} size={17} />
                      ) : paused ? (
                        <UiIcon icon={BatteryPauseIcon} size={17} />
                      ) : (
                        <UiIcon icon={DownloadIcon} size={17} />
                      )}
                    </span>
                    <span className="download-name">{truncate(it.name, 48)}</span>
                    <span className="download-meta">
                      {fmt.bytes(it.totalBytes)}
                      <span className="muted">{folderName(it.dir)}</span>
                      {ss && (
                        <span className="tag" style={{ color: ss.color }}>
                          {ss.tag}
                        </span>
                      )}
                    </span>
                  </div>
                  <TorrentProgress pct={it.progress} paused={paused || failed} pieceMap={it.pieceMap} />
                  {it.error && <p className="download-error">{it.error}</p>}
                  <div className="download-stats">
                    {failed ? (
                      <span className="muted">
                        {t("downloads.failed")} · {it.progress}%
                      </span>
                    ) : paused ? (
                      <span className="muted">
                        {t("downloads.paused")} · {it.progress}%
                      </span>
                    ) : (
                      <>
                        <span>{it.progress}%</span>
                        <span>{fmt.bytesPerSec(it.speed) || "…"}</span>
                        <span>· {t("downloads.peers", { count: it.peers })}</span>
                        {it.eta != null && it.eta > 0 && (
                          <span>· {fmt.eta(it.eta)}</span>
                        )}
                      </>
                    )}
                  </div>
                  <div className="download-actions">
                    {failed ? (
                      <button type="button" className="btn btn-primary" onClick={() => void retry(it.id)}>
                        {t("downloads.retry")}
                      </button>
                    ) : it.status === "downloading" ? (
                      <button type="button" className="btn btn-ghost" onClick={() => void pause(it.id)}>
                        {t("downloads.pause")}
                      </button>
                    ) : (
                      <button type="button" className="btn btn-ghost" onClick={() => void resume(it.id)}>
                        {t("downloads.resume")}
                      </button>
                    )}
                    <button type="button" className="btn btn-ghost" onClick={() => void revealFolder(it.dir)}>
                      {t("downloads.openFolder")}
                    </button>
                    <button type="button" className="btn btn-ghost" onClick={() => void cancelItem(it)}>
                      {t("downloads.cancel")}
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </Card>

      <Card
        title={t("downloads.history")}
        subtitle={history.length ? t("downloads.items", { count: history.length }) : undefined}
        className="history-card"
      >
        {history.length === 0 ? (
          <p className="empty empty--compact">{t("downloads.historyEmpty")}</p>
        ) : (
          <>
            <ul className="history-list">
              {history.map((h) => {
                const ss = h.source ? sourceStyle(h.source as never) : null;
                return (
                  <li key={h.id} className="history-item">
                    <div className="history-main">
                      <span className="history-name">{truncate(h.name, 52)}</span>
                      <span className="history-meta">
                        {fmt.bytes(h.sizeBytes)}
                        {ss && (
                          <span className="tag" style={{ color: ss.color }}>
                            {ss.tag}
                          </span>
                        )}
                        <span className="muted">· {fmt.relativeMs(h.completedAt)}</span>
                      </span>
                    </div>
                    <div className="download-actions">
                      <button
                        type="button"
                        className="btn btn-primary"
                        disabled={installingId === h.id}
                        onClick={() => void installRepack(h)}
                      >
                        {installingId === h.id ? t("library.installing") : t("library.installNow")}
                      </button>
                      <button type="button" className="btn btn-ghost" onClick={() => void revealFolder(h.dir)}>
                        {t("downloads.openFolder")}
                      </button>
                      <button type="button" className="btn btn-ghost" onClick={() => redownload(h)}>
                        {t("downloads.downloadAgain")}
                      </button>
                      <button
                        type="button"
                        className="btn btn-ghost"
                        onClick={async () => {
                          if (await confirmRemoveHistory(truncate(h.name, 40), confirm)) {
                            await removeHistory(h.id);
                          }
                        }}
                      >
                        {t("downloads.remove")}
                      </button>
                    </div>
                  </li>
                );
              })}
            </ul>
            <div className="history-footer">
              <button type="button" className="btn btn-ghost" onClick={() => void clearAll()}>
                {t("downloads.clearHistory")}
              </button>
            </div>
          </>
        )}
      </Card>
    </div>
  );
}
