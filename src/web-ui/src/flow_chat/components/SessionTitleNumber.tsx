import './SessionTitleNumber.scss';

/** Auxiliary identity stays outside the title's overflow and edit slots. */
export function SessionTitleNumber({ number }: { number?: string }) {
  return number ? (
    <span
      className="alex-session-title-number"
      data-alex-component="session-title-number"
      data-alex-part="root"
    >{number}</span>
  ) : null;
}
