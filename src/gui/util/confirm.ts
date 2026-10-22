import { message } from "@tauri-apps/plugin-dialog";
import type { Messages } from "../i18n/en";
import { format } from "../i18n/locale";

function yesNoCancel(m: Messages["confirm"]) {
  return { yes: m.yes, no: m.no, cancel: m.back };
}

export async function confirmCancelDownload(
  name: string,
  m: Messages["confirm"],
): Promise<"keep" | "delete" | null> {
  const result = await message(format(m.cancelDownloadMessage, { name }), {
    title: m.cancelDownloadTitle,
    kind: "warning",
    buttons: {
      yes: m.keepFiles,
      no: m.deleteFiles,
      cancel: m.back,
    },
  });
  if (result === m.keepFiles) return "keep";
  if (result === m.deleteFiles) return "delete";
  return null;
}

export async function confirmClearHistory(m: Messages["confirm"]): Promise<boolean> {
  const result = await message(m.clearHistoryMessage, {
    title: m.clearHistoryTitle,
    kind: "warning",
    buttons: yesNoCancel(m),
  });
  return result === m.yes;
}

export async function confirmRemoveHistory(
  name: string,
  m: Messages["confirm"],
): Promise<boolean> {
  const result = await message(format(m.removeHistoryMessage, { name }), {
    title: m.removeHistoryTitle,
    kind: "warning",
    buttons: yesNoCancel(m),
  });
  return result === m.yes;
}

export async function confirmRemoveGame(
  name: string,
  m: Messages["confirm"],
): Promise<"keep" | "delete" | null> {
  const result = await message(format(m.removeGameMessage, { name }), {
    title: m.removeGameTitle,
    kind: "warning",
    buttons: {
      yes: m.keepFiles,
      no: m.deleteFiles,
      cancel: m.back,
    },
  });
  if (result === m.keepFiles) return "keep";
  if (result === m.deleteFiles) return "delete";
  return null;
}
