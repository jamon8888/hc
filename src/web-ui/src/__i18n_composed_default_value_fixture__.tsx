import { useI18n } from '@/infrastructure/i18n';
export function I18nComposedDefaultValueFixture() {
  const { t } = useI18n('flow-chat');
  return <div>{t('flowChatHeader.turnBadge', { current: 1, defaultValue: `Turn ${1}` + ' fallback' })}</div>;
}
