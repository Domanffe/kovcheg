import type { ComponentType } from "react";
import type { IconProps } from "./types";
import BatteryPauseIcon from "./battery-pause-icon";
import ArkIcon from "./ark-icon";
import FlameIcon from "./flame-icon";
import DownloadIcon from "./download-icon";
import GamepadIcon from "./gamepad-icon";
import GearIcon from "./gear-icon";
import GlobeIcon from "./globe-icon";
import LibraryIcon from "./library-icon";
import MagnifierIcon from "./magnifier-icon";
import PlayerIcon from "./player-icon";
import RadioIcon from "./radio-icon";
import SparklesIcon from "./sparkles-icon";
import TriangleAlertIcon from "./triangle-alert-icon";
import UploadIcon from "./upload-icon";

export type IconComponent = ComponentType<IconProps>;

export function UiIcon({
  icon: Icon,
  size = 18,
  className = "",
}: {
  icon: IconComponent;
  size?: number;
  className?: string;
}) {
  return <Icon size={size} className={`ui-icon ${className}`.trim()} />;
}

export {
  ArkIcon,
  BatteryPauseIcon,
  FlameIcon,
  DownloadIcon,
  GamepadIcon,
  GearIcon,
  GlobeIcon,
  LibraryIcon,
  MagnifierIcon,
  PlayerIcon,
  RadioIcon,
  SparklesIcon,
  TriangleAlertIcon,
  UploadIcon,
};
