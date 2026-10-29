import React, { Suspense, lazy } from 'react';
import './ShellScene.scss';

const TerminalScene = lazy(() => import('../terminal/TerminalScene'));

interface ShellSceneProps {
  isActive?: boolean;
}

const ShellScene: React.FC<ShellSceneProps> = ({ isActive = true }) => (
  <div className="alex-shell-scene" data-testid="shell-panel" data-alex-scene="shell" data-alex-part="root" data-alex-state={isActive ? 'active' : undefined}>
    <Suspense fallback={<div className="alex-shell-scene__loading" data-alex-scene="shell" data-alex-part="loading" />}>
      <TerminalScene isActive={isActive} />
    </Suspense>
  </div>
);

export default ShellScene;
