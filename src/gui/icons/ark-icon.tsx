import { motion } from "motion/react";
import type { IconProps } from "./types";

/** The chest from the app icon: domed lid, seam, latch. */
export default function ArkIcon({
  size = 24,
  color = "currentColor",
  strokeWidth = 2,
  className = "",
}: IconProps) {
  return (
    <motion.svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`cursor-pointer ${className}`}
      style={{ overflow: "visible" }}
    >
      <path d="M3 11a9 9 0 0 1 18 0z" />
      <rect x="3" y="11" width="18" height="9" rx="1.5" />
      <path d="M10 11h4v4h-4z" />
    </motion.svg>
  );
}
