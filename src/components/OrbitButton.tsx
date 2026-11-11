import Icon from "./Icon";

/** Sends a panel's selection to the Órbita instead of removing it right away. */
export default function OrbitButton({ count, onClick }: { count: number; onClick: () => void }) {
  return (
    <button type="button" className="btn" onClick={onClick} title="Juntar com o resto e decidir tudo de uma vez">
      <Icon name="orbit" size={15} />
      <span>Pôr {count} na Órbita</span>
    </button>
  );
}
