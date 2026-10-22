import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { useToast } from "../components/Toast";
import { isTauri } from "../util/tauri";
import { errMsg } from "../util/errors";

export interface LibraryGame {
  id: string;
  title: string;
  dir: string;
  exe: string | null;
  cover: string | null;
  steamAppId: number | null;
  source: string | null;
  addedAt: number;
  lastPlayedAt: number | null;
  playtimeSecs: number;
  running: boolean;
}

export interface FolderProbe {
  installer: string | null;
  exe: string | null;
  silent: boolean;
}

export interface InstallProgress {
  title: string;
  phase: "running" | "manual" | "done" | "failed";
  message: string;
}

export function useLibrary() {
  const { toast } = useToast();
  const tauri = isTauri();
  const [games, setGames] = useState<LibraryGame[]>([]);
  const [installing, setInstalling] = useState<InstallProgress | null>(null);

  const run = useCallback(
    async (action: () => Promise<unknown>): Promise<void> => {
      if (!tauri) return;
      try {
        await action();
      } catch (e) {
        toast(errMsg(e), "error");
      }
    },
    [tauri, toast],
  );

  useEffect(() => {
    if (!tauri) return;
    void run(() => invoke<LibraryGame[]>("library_list").then(setGames));
    const offList = listen<LibraryGame[]>("library-changed", (e) => setGames(e.payload));
    const offInstall = listen<InstallProgress>("library-install", (e) => {
      const progress = e.payload;
      const finished = progress.phase === "done" || progress.phase === "failed";
      setInstalling(finished ? null : progress);
      if (progress.phase === "done") toast(progress.message, "success");
      if (progress.phase === "failed") toast(`${progress.title}: ${progress.message}`, "error");
    });
    return () => {
      void offList.then((fn) => fn());
      void offInstall.then((fn) => fn());
    };
  }, [run, tauri, toast]);

  const launch = useCallback(
    (id: string) => run(() => invoke("library_launch", { id })),
    [run],
  );

  const add = useCallback(
    (title: string, dir: string) => run(() => invoke("library_add", { title, dir, exe: null, source: null })),
    [run],
  );

  const install = useCallback(
    (dir: string, title: string, installDir?: string) =>
      run(() => invoke("library_install", { dir, title, installDir: installDir ?? null })),
    [run],
  );

  const remove = useCallback(
    (id: string, deleteFiles: boolean) => run(() => invoke("library_remove", { id, deleteFiles })),
    [run],
  );

  const setExe = useCallback(
    (id: string, exe: string) => run(() => invoke("library_set_exe", { id, exe })),
    [run],
  );

  const refreshCover = useCallback(
    (id: string) => run(() => invoke("library_refresh_cover", { id })),
    [run],
  );

  const openFolder = useCallback(
    (id: string) => run(() => invoke("library_open_folder", { id })),
    [run],
  );

  const probe = useCallback(
    async (dir: string): Promise<FolderProbe | null> => {
      if (!tauri) return null;
      try {
        return await invoke<FolderProbe>("library_probe", { dir });
      } catch (e) {
        toast(errMsg(e), "error");
        return null;
      }
    },
    [tauri, toast],
  );

  return { games, installing, tauri, launch, add, install, remove, setExe, refreshCover, openFolder, probe };
}
