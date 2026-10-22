/** Strips repack noise such as "[FitGirl Repack]" so cards and the Steam lookup stay readable. */
export function cleanTitle(name: string): string {
  const stripped = name
    .replace(/\[[^\]]*\]/g, " ")
    .replace(/\((?:repack|rip|rus|eng|multi\d*)[^)]*\)/gi, " ")
    .replace(/(steamrip|onlinefix|online-fix)\.\w+/gi, " ")
    .replace(/[._]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return stripped || name;
}
