import { useEffect, useRef, useState } from "react";
import { gb, type Report } from "../domain";
import Icon from "./Icon";

export default function ConfirmDialog({
  amount,
  onClose,
  onConfirm,
}: {
  amount: number;
  onClose: () => void;
  onConfirm: (destination: Report["destination"]) => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const [destination, setDestination] = useState<Report["destination"]>("trash");
  const [acknowledged, setAcknowledged] = useState(false);

  useEffect(() => {
    const dialog = ref.current;
    const previous = document.activeElement;
    dialog?.showModal();
    return () => {
      dialog?.close();
      if (previous instanceof HTMLElement) previous.focus({ preventScroll: true });
    };
  }, []);

  return (
    <dialog
      ref={ref}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      aria-labelledby="confirm-title"
      className="confirm-dialog"
    >
      <div className="section-heading">
        <span className="icon-tile">
          <Icon name="trash" size={24} />
        </span>
        <button type="button" className="icon-button" aria-label="Fechar confirmação" onClick={onClose}>
          <Icon name="close" />
        </button>
      </div>
      <p className="eyebrow">ÚLTIMA REVISÃO</p>
      <h2 id="confirm-title">Como você quer limpar?</h2>
      <p>{gb(amount)} selecionados para limpeza.</p>
      <label className={`destination-option ${destination === "trash" ? "chosen" : ""}`}>
        <input
          type="radio"
          name="destination"
          checked={destination === "trash"}
          onChange={() => setDestination("trash")}
        />
        <span>
          <strong>
            Enviar à Lixeira <span className="badge green">Recomendado</span>
          </strong>
          <small>Você pode restaurar pelo sistema. O espaço ainda não fica disponível.</small>
        </span>
      </label>
      <label className={`destination-option ${destination === "permanent" ? "chosen" : ""}`}>
        <input
          type="radio"
          name="destination"
          checked={destination === "permanent"}
          onChange={() => setDestination("permanent")}
        />
        <span>
          <strong>Excluir permanentemente</strong>
          <small>Sem restauração pela Lixeira. Libera espaço imediato.</small>
        </span>
      </label>
      {destination === "permanent" && (
        <label className="acknowledgment">
          <input type="checkbox" checked={acknowledged} onChange={(e) => setAcknowledged(e.target.checked)} />
          Entendo que a exclusão permanente não pode ser desfeita pela Lixeira.
        </label>
      )}
      <div className="dialog-actions">
        <button type="button" className="button secondary" onClick={onClose}>
          Voltar à revisão
        </button>
        <button
          type="button"
          className="button primary"
          disabled={destination === "permanent" && !acknowledged}
          onClick={() => onConfirm(destination)}
        >
          Limpar agora <Icon name="arrow" size={16} />
        </button>
      </div>
    </dialog>
  );
}
