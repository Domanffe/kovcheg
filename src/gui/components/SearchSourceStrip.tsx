import { useMemo } from "react";
import type { Section } from "../App";
import type { SourceState } from "../../core/search-state";
import { ACTIVE_SOURCE_IDS, getSourceMeta } from "../../sources/meta";
import type { SourceId } from "../../sources/types";
import { sourceStyle } from "../../sources/style";
import { CATEGORY_GROUP } from "../sections";
import { useLocale } from "../i18n/LocaleProvider";

type ChipStatus = "loading" | "done" | "error";

function chipStatus(st: SourceState | undefined): ChipStatus {
  if (!st || st.loading) return "loading";
  if (st.error) return "error";
  return "done";
}

export function SearchSourceStrip({
  perSource,
  category,
  active,
}: {
  perSource: Record<SourceId, SourceState>;
  category: Section;
  active: boolean;
}) {
  const { t } = useLocale();
  const sources = useMemo(() => {
    const group = CATEGORY_GROUP[category];
    return ACTIVE_SOURCE_IDS.filter((id) => !group || getSourceMeta(id).group === group);
  }, [category]);

  if (!active) return null;

  return (
    <div className="source-strip" role="status" aria-live="polite">
      {sources.map((id) => {
        const st = perSource[id];
        const status = chipStatus(st);
        const meta = getSourceMeta(id);
        const ss = sourceStyle(id);
        const title =
          status === "error"
            ? (st?.code ?? st?.error ?? undefined)
            : status === "done" && st && st.count > 0
              ? String(st.count)
              : undefined;
        return (
          <span
            key={id}
            className={`source-chip source-chip--${status}`}
            title={title}
            style={{ "--chip-color": ss.color } as React.CSSProperties}
          >
            <span className="source-chip-dot" aria-hidden />
            <span className="source-chip-tag">{ss.tag}</span>
            {meta.browseOnly && (
              <span className="source-chip-note">{t("search.browseOnly")}</span>
            )}
          </span>
        );
      })}
    </div>
  );
}
