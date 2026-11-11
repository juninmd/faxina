import { SPACES, type Space } from "../lib/spaces";
import Icon from "./Icon";

export default function Rail({
  space,
  badges,
  dots = {},
  onSpace,
  onPalette,
  motionLabel,
  onMotion,
}: {
  space: Space;
  badges: Partial<Record<Space, number>>;
  /** A small dot for news without a count, such as an available update. */
  dots?: Partial<Record<Space, boolean>>;
  onSpace: (s: Space) => void;
  onPalette: () => void;
  motionLabel: string;
  onMotion: () => void;
}) {
  return (
    <nav aria-label="Espaços" className="rail">
      <button type="button" className="rail-logo" onClick={() => onSpace("agora")} aria-label="Faxina, ir para Agora">
        <span aria-hidden="true" />
      </button>
      {SPACES.map((s) => (
        <button
          key={s.id}
          type="button"
          className="rail-item"
          aria-current={space === s.id ? "page" : undefined}
          onClick={() => onSpace(s.id)}
        >
          <Icon name={s.icon} size={22} />
          <span>{s.label}</span>
          {badges[s.id] ? (
            <span className="rail-badge">{badges[s.id]}</span>
          ) : (
            dots[s.id] && <span className="rail-dot" title="Nova versão disponível" />
          )}
        </button>
      ))}
      <div className="flex-1" />
      <button type="button" className="rail-item" onClick={onPalette} title="Buscar ou executar (Ctrl K)">
        <Icon name="search" size={20} />
        <span>Ctrl K</span>
      </button>
      <button
        type="button"
        className="rail-item"
        onClick={onMotion}
        title="Animações: seguir o sistema, sempre ou nunca"
        aria-label={`Animações: ${motionLabel}`}
      >
        <Icon name="spark" size={20} />
        <span>{motionLabel}</span>
      </button>
    </nav>
  );
}
