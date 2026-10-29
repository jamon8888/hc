 

import React from 'react';
import { Button, Icon, IconButton } from '@alex/ui';
import { AlertTriangle, XCircle } from 'lucide-react';
import { useI18n } from '@/infrastructure/i18n';
import { Notification } from '../types';
import { notificationService } from '../services/NotificationService';
import './NotificationItem.scss';

export interface NotificationItemProps {
  notification: Notification;
  isExiting?: boolean;
}

export const NotificationItem: React.FC<NotificationItemProps> = ({ notification, isExiting = false }) => {
  const { id, type, title, message, messageNode, closable, actions } = notification;
  const isAssertive = type === 'error' || type === 'warning';
  const { t } = useI18n('common');

  
  const getIcon = () => {
    switch (type) {
      case 'success':
        return <Icon name="check-circle" size="lg" />;
      case 'error':
        return <XCircle aria-hidden="true" />;
      case 'warning':
        return <AlertTriangle aria-hidden="true" />;
      case 'info':
      default:
        return <Icon name="info" size="lg" />;
    }
  };

  
  const handleClose = () => {
    notificationService.dismiss(id);
  };

  
  const handleAction = (onClick: () => void) => {
    onClick();
    
    if (closable) {
      notificationService.dismiss(id);
    }
  };

  return (
    <div
      className={`notification-item notification-item--${type}${closable ? ' notification-item--closable' : ''}${isExiting ? ' notification-item--exiting' : ''}`}
      data-alex-component="notification"
      data-alex-part="item"
      role={isAssertive ? 'alert' : 'status'}
      aria-live={isAssertive ? 'assertive' : 'polite'}
      aria-atomic="true"
    >
      
      <div className="notification-item__icon" data-alex-component="notification" data-alex-part="itemIcon">
        {getIcon()}
      </div>

      
      <div className="notification-item__content" data-alex-component="notification" data-alex-part="itemContent">
        <div className="notification-item__title" data-alex-component="notification" data-alex-part="itemTitle">{title}</div>
        <div className="notification-item__message" data-alex-component="notification" data-alex-part="itemMessage">{messageNode ?? message}</div>

        
        {actions && actions.length > 0 && (
          <div className="notification-item__actions" data-alex-component="notification" data-alex-part="itemActions">
            {actions.map((action, index) => (
              <Button
                key={index}
                variant={action.variant === 'primary' ? 'primary' : action.variant === 'danger' ? 'fill' : 'outline'}
                tone={action.variant === 'danger' ? 'danger' : 'neutral'}
                size="sm"
                onClick={() => handleAction(action.onClick)}
              >
                {action.label}
              </Button>
            ))}
          </div>
        )}
      </div>

      
      {closable && (
        <span
          className="notification-item__close"
          data-alex-component="notification"
          data-alex-part="itemClose"
        >
          <IconButton
            shape="circle"
            size="xs"
            variant="fill"
            onClick={handleClose}
            aria-label={t('actions.close')}
            icon={<Icon name="xmark" size="sm" />}
          />
        </span>
      )}
    </div>
  );
};
