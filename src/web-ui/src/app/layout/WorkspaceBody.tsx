/**
 * WorkspaceBody — main workspace container.
 *
 * Left-right layout:
 *   .nav-area   (300px default, flex-column)
 *     NavBar        (45px — back/forward + drag + WindowControls)
 *     NavPanel      (flex:1 — navigation sidebar)
 *   .scene-area (flex:1, flex-column)
 *     SceneTopBar   (scene tabs + active-scene chrome + WindowControls)
 *     SceneViewport (flex:1 — active scene content)
 */

import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useCurrentWorkspace } from '../../infrastructure/contexts/WorkspaceContext';
import { NavBar } from '../components/NavBar';
import NavPanel from '../components/NavPanel/NavPanel';
import { SceneChromeProvider, SceneTopBar } from '../components/SceneTopBar';
import { SceneViewport } from '../scenes';
import TerminalActionBridge from '../scenes/terminal/TerminalActionBridge';
import { useApp } from '../hooks/useApp';
import { selectActiveSceneId, useSceneStore } from '../stores/sceneStore';
import './WorkspaceBody.scss';

const NAV_DEFAULT_WIDTH = 300;
const NAV_MIN_WIDTH = 216;
const NAV_MAX_WIDTH = 480;
const COLLAPSE_THRESHOLD = 64;

interface WorkspaceBodyProps {
  className?: string;
  isEntering?: boolean;
  isExiting?: boolean;
  onMinimize?: () => void;
  onMaximize?: () => void;
  onClose?: () => void;
  isMaximized?: boolean;
  sceneOverlay?: React.ReactNode;
}

const WorkspaceBody: React.FC<WorkspaceBodyProps> = ({
  className = '',
  isEntering = false,
  isExiting = false,
  onMinimize,
  onMaximize,
  onClose,
  isMaximized = false,
  sceneOverlay,
}) => {
  const { workspace: currentWorkspace } = useCurrentWorkspace();
  const { state, toggleLeftPanel } = useApp();
  const activeSceneId = useSceneStore(selectActiveSceneId);
  const isNavCollapsed = state.layout.leftPanelCollapsed;
  const [navWidth, setNavWidth] = useState(NAV_DEFAULT_WIDTH);
  const navAreaRef = useRef<HTMLDivElement>(null);
  const navDividerRef = useRef<HTMLDivElement>(null);
  // Active drag cleanup, so window listeners never leak if we unmount mid-drag.
  const dragCleanupRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    return () => {
      dragCleanupRef.current?.();
    };
  }, []);

  const handleNavCollapseDragStart = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0 || isNavCollapsed) return;
    event.preventDefault();

    const startX = event.clientX;
    const startWidth = navWidth;
    let latestWidth = startWidth;
    let hasCollapsed = false;
    let frameId: number | null = null;

    document.body.classList.add('alex-is-dragging-nav-collapse');
    document.body.classList.add('alex-is-resizing-nav');

    // During the drag we bypass React entirely: write the --nav-width CSS
    // variable straight to the two elements that consume it (rAF-merged).
    // React state is committed once on mouseup.
    const applyWidth = () => {
      frameId = null;
      const value = `${latestWidth}px`;
      navAreaRef.current?.style.setProperty('--nav-width', value);
      navDividerRef.current?.style.setProperty('--nav-width', value);
    };

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (hasCollapsed) return;
      const deltaX = moveEvent.clientX - startX;
      const rawWidth = startWidth + deltaX;

      // Collapse only after the width hits minimum AND continues left by COLLAPSE_THRESHOLD
      if (rawWidth <= NAV_MIN_WIDTH - COLLAPSE_THRESHOLD) {
        hasCollapsed = true;
        toggleLeftPanel();
        cleanup();
        return;
      }
      latestWidth = Math.min(NAV_MAX_WIDTH, Math.max(NAV_MIN_WIDTH, rawWidth));
      if (frameId === null) {
        frameId = requestAnimationFrame(applyWidth);
      }
    };

    const handleMouseUp = () => cleanup();

    function cleanup() {
      if (frameId !== null) {
        cancelAnimationFrame(frameId);
        frameId = null;
      }
      // Flush the final width synchronously, while `alex-is-resizing-nav`
      // still suppresses the width transition. Without this, a drop that lands
      // between two animation frames leaves the DOM at the last painted width:
      // React only rewrites the inline var when `navWidth` actually changes, so
      // ending a drag back on the previous width would strand the DOM out of
      // sync with state, and any other drop would glide to its final width
      // ($motion-base) while the divider's `left` jumps instantly.
      applyWidth();
      document.body.classList.remove('alex-is-dragging-nav-collapse');
      document.body.classList.remove('alex-is-resizing-nav');
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      dragCleanupRef.current = null;
      // Commit the final width through React once.
      setNavWidth(latestWidth);
    }

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    dragCleanupRef.current = cleanup;
  }, [isNavCollapsed, navWidth, toggleLeftPanel]);

  return (
    <div
      className={`alex-workspace-body${isEntering ? ' is-entering' : ''}${isExiting ? ' is-exiting' : ''} ${className}`}
      data-alex-scene="workbench"
      data-alex-part="workspace"
      data-alex-state={isNavCollapsed ? 'collapsed' : undefined}
    >
      <div
        className="alex-workspace-body__material"
        data-alex-scene="workbench"
        data-alex-part="workspaceMaterial"
        data-alex-theme-scope="chrome"
        aria-hidden="true"
      />
      {isNavCollapsed && (
        <div className="alex-workspace-body__collapsed-nav" data-alex-scene="workbench" data-alex-part="collapsedNav">
          <NavBar isCollapsed onExpandNav={toggleLeftPanel} onMaximize={onMaximize} />
        </div>
      )}

      {/* Left: nav history bar + navigation sidebar — always rendered for slide animation */}
      <div
        ref={navAreaRef}
        className={`alex-workspace-body__nav-area${isNavCollapsed ? ' is-collapsed' : ''}`}
        style={isNavCollapsed ? undefined : { '--nav-width': `${navWidth}px` } as React.CSSProperties}
        data-alex-scene="workbench"
        data-alex-part="navArea"
        data-alex-theme-scope="chrome"
        data-alex-state={isNavCollapsed ? 'collapsed' : undefined}
      >
        <NavBar onExpandNav={toggleLeftPanel} onMaximize={onMaximize} />
        <NavPanel className="alex-workspace-body__nav-panel" />
        <div data-alex-creation-slot="sidebar-footer" />
      </div>

      <TerminalActionBridge />

      {/* Resize divider — placed at workspace-body level to avoid overflow:hidden clipping */}
      {!isNavCollapsed && (
        <div
          ref={navDividerRef}
          className="alex-workspace-body__nav-divider"
          style={{ '--nav-width': `${navWidth}px` } as React.CSSProperties}
          onMouseDown={handleNavCollapseDragStart}
          role="separator"
          aria-hidden="true"
          data-alex-scene="workbench"
          data-alex-part="navDivider"
        />
      )}

      {/* Right: visual scene surface + any shell-level overlay */}
      <div className="alex-workspace-body__scene-area" data-alex-scene="workbench" data-alex-part="sceneArea">
        <div
          className="alex-workspace-body__scene-surface"
          data-alex-scene="workbench"
          data-alex-part="sceneSurface"
        >
          <SceneChromeProvider activeSceneId={activeSceneId}>
            <div data-alex-creation-slot="scene-header" />
            <SceneTopBar
              onMinimize={onMinimize}
              onMaximize={onMaximize}
              onClose={onClose}
              isMaximized={isMaximized}
            />
            <SceneViewport
              workspacePath={currentWorkspace?.rootPath}
              isEntering={isEntering}
            />
            <div data-alex-creation-slot="scene-footer" />
          </SceneChromeProvider>
        </div>
        {sceneOverlay}
      </div>
    </div>
  );
};

export default WorkspaceBody;
