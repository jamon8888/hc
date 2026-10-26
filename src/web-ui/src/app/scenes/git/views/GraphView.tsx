/**
 * GraphView — Wraps GitGraphView for the Git scene graph tab.
 */

import React from 'react';
import { GitGraphView } from '@/tools/git/components/GitGraphView';
import './GraphView.scss';

interface GraphViewProps {
  workspacePath?: string;
  workspaceId?: string;
}

const GraphView: React.FC<GraphViewProps> = ({ workspacePath = '', workspaceId }) => {
  if (!workspacePath) {
    return (
      <div data-alex-component="git-graph-view" data-alex-part="root" data-alex-state="empty" className="alex-git-scene-graph alex-git-scene-graph--empty">
        <p>Open a workspace to see the commit graph.</p>
      </div>
    );
  }

  return (
    <div data-alex-component="git-graph-view" data-alex-part="root" className="alex-git-scene-graph">
      <GitGraphView repositoryPath={{ workspaceId: workspaceId ?? '', repositoryPath: workspacePath }} />
    </div>
  );
};

export default GraphView;
