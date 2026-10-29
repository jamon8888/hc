import { useHasAppUpdate } from './useHasAppUpdate';
import './UpdateIndicator.scss';

export function UpdateIndicator() {
  const visible = useHasAppUpdate();
  return visible ? <span className="alex-update-indicator" aria-hidden="true"
    data-testid="app-update-indicator" data-alex-component="update" data-alex-part="indicator" /> : null;
}
