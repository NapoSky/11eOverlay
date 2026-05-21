export interface ContentItem {
  key: string;
  value: string;
}

export interface ContentSection {
  section: string;
  items: ContentItem[];
}

export interface AppSettings {
  hotkey: string;
  /** Background opacity 0–1 */
  opacity: number;
  width: number;
  height: number;
  x: number;
  y: number;
  /** Content font size in px */
  fontSize: number;
  /** Last active section chip (null = "Tout") */
  activeSection: string | null;
  /** Active overlay mode: "lexique" | "artillerie". */
  activeMode: string;
  /** Last selected ArtyCon instance id. */
  artyInstanceId: string | null;
  /** Last selected layer id. */
  artyLayerId: string | null;
  /** Last selected group id. */
  artyGroupId: string | null;
  /** Last selected battery id. */
  artyBatteryId: string | null;
  /** When true, the artillery panel shows solutions for all batteries of the group. */
  artyShowAllBatteries: boolean;
}
