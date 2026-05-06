/** Props required by every panel rendered from the PanelRegistry. */
export interface PanelProps {
  onClose: () => void;
  initialArgs?: string;
}
