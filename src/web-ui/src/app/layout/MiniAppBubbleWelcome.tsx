import React from 'react';
import { OverflowText, Icon, ScrollArea } from '@alex/ui';
import { FolderOpen } from 'lucide-react';
import type { MiniAppBubbleCustomization } from '@/app/scenes/miniapps/miniAppStore';
import { renderMiniAppIcon } from '@/app/scenes/miniapps/utils/miniAppIcons';
import { useChatInputState } from '@/flow_chat/store/chatInputStateStore';
import { computeFlowChatInputStackFooterPx } from '@/flow_chat/utils/flowChatScrollLayout';

interface MiniAppBubbleWelcomeProps {
  appName: string;
  appDescription?: string;
  appIcon?: string;
  showIcon?: boolean;
  customization?: MiniAppBubbleCustomization;
  workspacePath?: string;
  onSuggestion: (prompt: string) => void;
}

/** Keep aligned with the block padding on `.alex-fmc__miniapp-welcome-content`. */
const WELCOME_CONTENT_BLOCK_PADDING_PX = 36;

/**
 * Host-rendered empty state for an Agentic MiniApp session. MiniApps provide a
 * bounded declarative model through app.chat.claimComposer; they never inject
 * markup into the shared conversation surface.
 */
export const MiniAppBubbleWelcome: React.FC<MiniAppBubbleWelcomeProps> = ({
  appName,
  appDescription,
  appIcon = 'Box',
  showIcon = true,
  customization,
  workspacePath,
  onSuggestion,
}) => {
  const welcome = customization?.welcome;
  const title = welcome?.title || appName;
  const description = welcome?.description || appDescription;
  const workspaceLabel = welcome?.workspaceLabel || (workspacePath ? appName : '');
  const suggestions = welcome?.suggestions || [];
  const inputHeight = useChatInputState(state => state.inputHeight);
  const inputClearance = computeFlowChatInputStackFooterPx(inputHeight);

  return (
    <ScrollArea className="alex-fmc__miniapp-welcome" data-alex-component="miniapp-bubble-welcome" data-alex-part="root">
      <div
        className="alex-fmc__miniapp-welcome-content"
        style={{
          paddingBottom: `${WELCOME_CONTENT_BLOCK_PADDING_PX + inputClearance}px`,
        }}
      >
        {showIcon && <div
          className="alex-fmc__miniapp-welcome-icon"
          data-alex-component="miniapp-bubble-welcome"
          data-alex-part="icon"
          aria-hidden="true"
        >
          {renderMiniAppIcon(appIcon, 28)}
        </div>}

        {title !== appName && (
          <div className="alex-fmc__miniapp-welcome-eyebrow" data-alex-component="miniapp-bubble-welcome" data-alex-part="eyebrow">{appName}</div>
        )}
        <h2 data-alex-component="miniapp-bubble-welcome" data-alex-part="title">{title}</h2>
        {description && <p className="alex-fmc__miniapp-welcome-description" data-alex-component="miniapp-bubble-welcome" data-alex-part="description">{description}</p>}

        {workspaceLabel && workspacePath && (
          <div
            className="alex-fmc__miniapp-workspace"
            data-alex-component="miniapp-bubble-welcome"
            data-alex-part="workspace"
            title={workspacePath}
            data-workspace-path={workspacePath}
          >
            <FolderOpen size={13} aria-hidden="true" />
            <OverflowText>{workspaceLabel}</OverflowText>
          </div>
        )}

        {suggestions.length > 0 && (
          <div className="alex-fmc__miniapp-suggestions" data-alex-component="miniapp-bubble-welcome" data-alex-part="suggestions">
            {welcome?.suggestionsLabel && (
              <div className="alex-fmc__miniapp-suggestions-label" data-alex-component="miniapp-bubble-welcome" data-alex-part="suggestionsLabel">
                {welcome.suggestionsLabel}
              </div>
            )}
            <div className="alex-fmc__miniapp-suggestions-list" data-alex-component="miniapp-bubble-welcome" data-alex-part="suggestionsList">
              {suggestions.map((suggestion, index) => (
                <button
                  key={`${suggestion.label}:${index}`}
                  type="button"
                  className="alex-fmc__miniapp-suggestion"
                  data-alex-component="miniapp-bubble-welcome"
                  data-alex-part="suggestion"
                  title={suggestion.prompt}
                  onClick={() => onSuggestion(suggestion.prompt)}
                >
                  <span>{suggestion.label}</span>
                  <Icon name="arrow-up-right" size="xs" aria-hidden="true" />
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </ScrollArea>
  );
};

export default MiniAppBubbleWelcome;
