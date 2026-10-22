const SKIP_SPLASH_KEY = "kovcheg.skipSplash";

export function shouldSkipSplash(): boolean {
  try {
    return localStorage.getItem(SKIP_SPLASH_KEY) === "1";
  } catch {
    return false;
  }
}

export function markSplashSeen(): void {
  try {
    localStorage.setItem(SKIP_SPLASH_KEY, "1");
  } catch {
    /* ignore */
  }
}
