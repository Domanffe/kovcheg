import { isInfoHash } from "../sources/magnet";

export type CliCommand =
  | { kind: "version" }
  | { kind: "help" }
  | { kind: "gui" }
  | { kind: "search" }
  | { kind: "invalid"; arg: string };

export function parseCliArgs(argv: string[]): CliCommand {
  const args = argv.filter((a) => a.trim() !== "");
  if (args.length === 0) return { kind: "gui" };
  const a = args[0]!;
  if (a === "--version" || a === "-v") return { kind: "version" };
  if (a === "--help" || a === "-h") return { kind: "help" };
  if (a === "--search") return { kind: "search" };
  if (a === "--gui") return { kind: "gui" };
  if (/^magnet:\?/i.test(a)) return { kind: "gui" };
  if (isInfoHash(a)) return { kind: "gui" };
  if (/\.torrent$/i.test(a)) return { kind: "gui" };
  return { kind: "invalid", arg: a };
}

export const HELP_TEXT = `kovcheg — torrent search & download

usage
  kovcheg                      open the browser search UI (or use the desktop app)
  kovcheg --gui                same as default
  kovcheg --search             search API server only (dev)
  kovcheg "magnet:?xt=..."     open the GUI (magnet handling: desktop app)
  kovcheg path/to/file.torrent open the GUI (.torrent handling: desktop app)
  kovcheg --version            print the version

desktop app: install from GitHub Releases for full download/seeding (librqbit)
tip: quote magnet links (they contain & characters)
`;
