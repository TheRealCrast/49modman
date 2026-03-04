import broken from "./icons/broken.svg";
import check from "./icons/check-confirm.svg";
import circle from "./icons/circle.svg";
import details from "./icons/details.svg";
import download from "./icons/download.svg";
import edit from "./icons/edit.svg";
import externalLink from "./icons/external-link.svg";
import filter from "./icons/filter.svg";
import folder from "./icons/folder.svg";
import home from "./icons/home.svg";
import play from "./icons/play.svg";
import profiles from "./icons/profiles.svg";
import refresh from "./icons/refresh.svg";
import search from "./icons/search.svg";
import settings from "./icons/settings.svg";
import threeDotsVertical from "./icons/three-dots-vertical.svg";
import verified from "./icons/verified.svg";
import warning from "./icons/alert.svg";
import xClose from "./icons/x-close.svg";

export type IconName =
  | "broken"
  | "check"
  | "circle"
  | "details"
  | "download"
  | "edit"
  | "external-link"
  | "filter"
  | "folder"
  | "home"
  | "play"
  | "profiles"
  | "refresh"
  | "search"
  | "settings"
  | "three-dots-vertical"
  | "verified"
  | "warning"
  | "x-close";

export const iconMap: Record<IconName, string> = {
  broken,
  check,
  circle,
  details,
  download,
  edit,
  "external-link": externalLink,
  filter,
  folder,
  home,
  play,
  profiles,
  refresh,
  search,
  settings,
  "three-dots-vertical": threeDotsVertical,
  verified,
  warning,
  "x-close": xClose
};
