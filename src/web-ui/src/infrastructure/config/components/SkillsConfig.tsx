/* eslint-disable @typescript-eslint/no-use-before-define */
import { OverflowText, Button, Card, CardBody, ConfirmDialog, Field, Icon, IconButton, Input, SearchField, Select, Tooltip } from '@alex/ui';
import React, { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { FolderOpen, TrendingUp } from 'lucide-react';

import { useI18n } from '@/infrastructure/i18n/hooks/useI18n';
import { installedSkillMarketIds, isSkillMarketItemInstalled } from '@/infrastructure/config/skillMarketInstallation';

import { ConfigPageHeader, ConfigPageLayout, ConfigPageContent, ConfigPageSection, ConfigCollectionItem } from './common';
import { useCurrentWorkspace } from '@/infrastructure/contexts/WorkspaceContext';
import { useNotification } from '@/shared/notification-system';
import { isRemoteWorkspace } from '@/shared/types';
import { configAPI } from '../../api/service-api/ConfigAPI';
import type { SkillInfo, SkillLevel, SkillMarketItem, SkillValidationResult } from '../types';
import {
  buildSkillCoverageSourceMap,
  canDeleteSkill,
  getSkillSourceLabel,
} from '../skillSourcePresentation';
import { open } from '@tauri-apps/plugin-dialog';
import { createLogger } from '@/shared/utils/logger';
import './SkillsConfig.scss';

const log = createLogger('SkillsConfig');

const SkillsConfig: React.FC = () => {
  const { t } = useTranslation('settings/skills');
  const { t: tShared } = useI18n(['components', 'common']);
  const [showAddForm, setShowAddForm] = useState(false);
  const [expandedSkillIds, setExpandedSkillIds] = useState<Set<string>>(new Set());
  const [skills, setSkills] = useState<SkillInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [formLevel, setFormLevel] = useState<SkillLevel>('user');
  const [formPath, setFormPath] = useState('');
  const [validationResult, setValidationResult] = useState<SkillValidationResult | null>(null);
  const [isValidating, setIsValidating] = useState(false);
  const [isAdding, setIsAdding] = useState(false);

  const [deleteConfirm, setDeleteConfirm] = useState<{ show: boolean; skill: SkillInfo | null }>({
    show: false,
    skill: null,
  });

  const [marketKeyword, setMarketKeyword] = useState('');
  const [marketSkills, setMarketSkills] = useState<SkillMarketItem[]>([]);
  const [marketLoading, setMarketLoading] = useState(false);
  const [marketError, setMarketError] = useState<string | null>(null);
  const [downloadingPackage, setDownloadingPackage] = useState<string | null>(null);
  const loadRequestIdRef = useRef(0);
  const coverageSourceBySkillKey = useMemo(
    () => buildSkillCoverageSourceMap(skills, t('list.item.unknownSource')),
    [skills, t],
  );

  const { workspace, workspacePath, hasWorkspace } = useCurrentWorkspace();
  const isRemote = isRemoteWorkspace(workspace);
  const notification = useNotification();

  const loadSkills = useCallback(async (forceRefresh?: boolean) => {
    const requestId = ++loadRequestIdRef.current;

    try {
      setLoading(true);
      setError(null);
      const skillsList = await configAPI.getSkillConfigs({
        forceRefresh,
        workspaceId: workspace?.id,
      });
      if (requestId !== loadRequestIdRef.current) {
        return;
      }
      setSkills(skillsList);
    } catch (err) {
      if (requestId !== loadRequestIdRef.current) {
        return;
      }
      log.error('Failed to load skills', err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (requestId === loadRequestIdRef.current) {
        setLoading(false);
      }
    }
  }, [workspace?.id]);

  const loadMarketSkills = useCallback(async (query?: string) => {
    try {
      setMarketLoading(true);
      setMarketError(null);
      const normalized = query?.trim();
      const skillList = normalized
        ? await configAPI.searchSkillMarket(normalized, 20)
        : await configAPI.listSkillMarket(undefined, 20);
      setMarketSkills(skillList);
    } catch (err) {
      log.error('Failed to load skill market', err);
      setMarketError(err instanceof Error ? err.message : String(err));
    } finally {
      setMarketLoading(false);
    }
  }, []);

  useEffect(() => { loadSkills(); }, [loadSkills]);
  useEffect(() => { loadMarketSkills(); }, [loadMarketSkills]);

  const validatePath = useCallback(async (path: string) => {
    if (!path.trim()) { setValidationResult(null); return; }
    try {
      setIsValidating(true);
      const result = await configAPI.validateSkillPath(path);
      setValidationResult(result);
    } catch (err) {
      setValidationResult({ valid: false, error: err instanceof Error ? err.message : String(err) });
    } finally {
      setIsValidating(false);
    }
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => { validatePath(formPath); }, 300);
    return () => clearTimeout(timer);
  }, [formPath, validatePath]);

  const handleAdd = async () => {
    if (!validationResult?.valid || !formPath.trim()) {
      notification.warning(t('messages.invalidPath'));
      return;
    }
    if (formLevel === 'project' && !hasWorkspace) {
      notification.warning(t('messages.noWorkspace'));
      return;
    }
    try {
      setIsAdding(true);
      await configAPI.addSkill({
        sourcePath: formPath,
        level: formLevel,
        workspaceId: workspace?.id,
      });
      notification.success(t('messages.addSuccess', { name: validationResult.name }));
      resetForm();
      await loadSkills(true);
    } catch (err) {
      notification.error(t('messages.addFailed', { error: err instanceof Error ? err.message : String(err) }));
    } finally {
      setIsAdding(false);
    }
  };

  const confirmDelete = async () => {
    const skill = deleteConfirm.skill;
    if (!skill || !canDeleteSkill(skill)) {
      setDeleteConfirm({ show: false, skill: null });
      return;
    }
    try {
      await configAPI.deleteSkill({
        skillKey: skill.key,
        workspaceId: workspace?.id,
      });
      notification.success(t('messages.deleteSuccess', { name: skill.name }));
      await loadSkills(true);
    } catch (err) {
      notification.error(t('messages.deleteFailed', { error: err instanceof Error ? err.message : String(err) }));
    } finally {
      setDeleteConfirm({ show: false, skill: null });
    }
  };

  const handleDownload = async (skill: SkillMarketItem, targetLevel: SkillLevel = 'project') => {
    const resolvedLevel: SkillLevel = isRemote ? 'user' : targetLevel;
    if (resolvedLevel === 'project' && !hasWorkspace) {
      notification.warning(t('messages.noWorkspace'));
      return;
    }

    try {
      setDownloadingPackage(skill.installId);
      const result = await configAPI.downloadSkillMarket({
        packageId: skill.installId,
        level: resolvedLevel,
        workspaceId: resolvedLevel === 'project' ? workspace?.id : undefined,
      });
      const installedName = result.installedSkills[0] ?? skill.name;
      notification.success(t('messages.marketDownloadSuccess', { name: installedName }));
      await loadSkills(true);
    } catch (err) {
      notification.error(t('messages.marketDownloadFailed', { error: err instanceof Error ? err.message : String(err) }));
    } finally {
      setDownloadingPackage(null);
    }
  };

  const handleBrowse = async () => {
    try {
      const selected = await open({ directory: true, multiple: false, title: t('form.path.label') });
      if (selected) setFormPath(selected as string);
    } catch (err) {
      log.error('Failed to open file dialog', err);
    }
  };

  const resetForm = () => {
    setFormPath('');
    setFormLevel('user');
    setValidationResult(null);
    setShowAddForm(false);
  };

  const toggleSkillExpanded = (skillId: string) => {
    setExpandedSkillIds(prev => {
      const next = new Set(prev);
      if (next.has(skillId)) next.delete(skillId);
      else next.add(skillId);
      return next;
    });
  };

  const renderAddForm = (level: SkillLevel) => {
    if (!showAddForm || formLevel !== level) return null;
    return (
      <div className="alex-collection-form" data-alex-component="skills-config" data-alex-part="form">
        <div className="alex-collection-form__header">
          <h3>{t('form.title')}</h3>
          <Tooltip content={t('form.closeTooltip')}>
            <IconButton
              aria-label={t('form.closeTooltip')}
              size="sm"
              onClick={resetForm}
              icon={<Icon name="xmark" size="sm" />}
            />
          </Tooltip>
        </div>
        <div className="alex-collection-form__body" data-alex-component="skills-config" data-alex-part="formBody">
          <Field label={t('form.level.label')} controlWidth="fill">
            <Select
              options={[
                { label: t('form.level.user'), value: 'user' },
                {
                  label: `${t('form.level.project')}${!hasWorkspace ? t('form.level.projectDisabled') : ''}`,
                  value: 'project',
                  disabled: !hasWorkspace
                }
              ]}
              value={formLevel}
              onValueChange={(value) => setFormLevel(value as SkillLevel)}
              size="sm"
            />
          </Field>
          {formLevel === 'project' && hasWorkspace && (
            <div className="alex-skills-config__form-hint">
              {t('form.level.currentWorkspace', { path: workspacePath })}
            </div>
          )}
          <div className="alex-skills-config__path-input">
            <Field label={t('form.path.label')} controlWidth="fill">
              <Input
                placeholder={t('form.path.placeholder')}
                value={formPath}
                onChange={(e) => setFormPath(e.target.value)}
              />
            </Field>
            <Tooltip content={t('form.path.browseTooltip')}>
              <IconButton
                aria-label={t('form.path.browseTooltip')}
                size="sm"
                onClick={handleBrowse}
                icon={<FolderOpen size={16} />}
              />
            </Tooltip>
          </div>
          <div className="alex-skills-config__path-hint">{t('form.path.hint')}</div>
          {isValidating && <div className="alex-skills-config__validating">{t('form.validating')}</div>}
          {validationResult && (
            <div className={`alex-skills-config__validation ${validationResult.valid ? 'is-valid' : 'is-invalid'}`}>
              {validationResult.valid ? (
                <>
                  <div className="alex-skills-config__validation-name">✓ {validationResult.name}</div>
                  <div className="alex-skills-config__validation-desc">{validationResult.description}</div>
                </>
              ) : (
                <div className="alex-skills-config__validation-error">✗ {validationResult.error}</div>
              )}
            </div>
          )}
        </div>
        <div className="alex-collection-form__footer">
          <Button variant="fill" size="sm" onClick={resetForm}>
            {t('form.actions.cancel')}
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={handleAdd}
            disabled={!validationResult?.valid || isAdding}
          >
            {isAdding ? t('form.actions.adding') : t('form.actions.add')}
          </Button>
        </div>
      </div>
    );
  };

  const renderSkillRow = (skill: SkillInfo) => {
    const sourceLabel = getSkillSourceLabel(skill, t('list.item.unknownSource'));
    const coverageSourceLabel = coverageSourceBySkillKey.get(skill.key);
    const badge = (
      <>
        <span className="alex-collection-item__badge">
          {isRemote
            ? skill.level === 'user'
              ? t('list.item.localUser')
              : t('list.item.remoteProject')
            : skill.level === 'user'
              ? t('list.item.user')
              : t('list.item.project')}
        </span>
        <span className="alex-collection-item__badge alex-skills-config__source-badge">
          {sourceLabel}
        </span>
        {skill.isShadowed && (
          <span
            className="alex-collection-item__badge alex-skills-config__covered-badge"
            title={t('list.item.shadowedTooltip', {
              source: coverageSourceLabel ?? t('list.item.unknownSource'),
            })}
          >
            {t('list.item.shadowed')}
          </span>
        )}
      </>
    );
    const control = canDeleteSkill(skill) ? (
        <IconButton
          type="button"
          className="alex-collection-btn alex-collection-btn--danger"
          onClick={() => setDeleteConfirm({ show: true, skill })}
          title={t('list.item.deleteTooltip')}
          aria-label={t('list.item.deleteTooltip')}
          icon={<Icon name="delete" size="sm" />}
        />
    ) : null;
    const details = (
      <div data-alex-component="skills-config" data-alex-part="details">
        <div className="alex-collection-details__field">{skill.description}</div>
        <div className="alex-collection-details__meta">
          <span className="alex-collection-details__label">{t('list.item.sourceLabel')}</span>
          <span>{sourceLabel}</span>
        </div>
        {skill.isShadowed && (
          <div className="alex-collection-details__meta alex-skills-config__coverage-detail">
            <span className="alex-collection-details__label">{t('list.item.shadowedLabel')}</span>
            <span>
              {t('list.item.shadowedDetail', {
                source: coverageSourceLabel ?? t('list.item.unknownSource'),
              })}
            </span>
          </div>
        )}
        <div className="alex-collection-details__meta">
          <span className="alex-collection-details__label">{t('list.item.pathLabel')}</span>
          <code className="alex-skills-config__path-value">{skill.path}</code>
        </div>
      </div>
    );
    return (
      <ConfigCollectionItem
        key={skill.key}
        label={skill.name}
        badge={badge}
        badgePlacement="below"
        control={control}
        details={details}
        expanded={expandedSkillIds.has(skill.key)}
        onToggle={() => toggleSkillExpanded(skill.key)}
        className={skill.isShadowed ? 'alex-skills-config__item--covered' : undefined}
        data-alex-component="skills-config"
        data-alex-part="item"
        data-alex-state={skill.isShadowed ? 'covered' : undefined}
      />
    );
  };

  const renderMarketList = () => {
    if (marketLoading) {
      return (
        <div className="alex-skills-config__market-list" aria-busy="true" aria-label={t('market.loading')} data-alex-component="skills-config" data-alex-part="marketList" data-alex-state="loading">
          {Array.from({ length: 5 }).map((_, index) => (
            <Card
              key={`market-loading-${index}`}
              appearance="raised"
              padding="none"
              className="alex-skills-config__market-item is-loading"
              data-alex-component="skills-config"
              data-alex-part="marketItem"
              data-alex-state="loading"
            >
              <CardBody className="alex-skills-config__market-item-body">
                <div className="alex-skills-config__market-skeleton-main">
                  <div className="alex-skills-config__market-skeleton-line alex-skills-config__market-skeleton-line--title" />
                  <div className="alex-skills-config__market-skeleton-line alex-skills-config__market-skeleton-line--desc" />
                  <div className="alex-skills-config__market-skeleton-line alex-skills-config__market-skeleton-line--desc is-short" />
                  <div className="alex-skills-config__market-skeleton-chip" />
                </div>
                <div className="alex-skills-config__market-skeleton-btn" />
              </CardBody>
            </Card>
          ))}
        </div>
      );
    }

    if (marketError) {
      return <div className="alex-skills-config__market-state alex-skills-config__market-state--error" data-alex-component="skills-config" data-alex-part="marketState" data-alex-state="error">{t('market.errorPrefix')}{marketError}</div>;
    }

    if (marketSkills.length === 0) {
      return (
        <div className="alex-skills-config__market-state" data-alex-component="skills-config" data-alex-part="marketState">
          {marketKeyword.trim() ? t('market.empty.noMatch') : t('market.empty.noSkills')}
        </div>
      );
    }

    return (
      <div className="alex-skills-config__market-list" data-alex-component="skills-config" data-alex-part="marketList">
        {displayMarketSkills.map((skill) => {
          const isDownloading = downloadingPackage === skill.installId;
          const isInstalled = isSkillMarketItemInstalled(skill, installedMarketIds);
          const sourceLabel = formatMarketSource(skill.source);
          const projectTooltipText = !hasWorkspace
            ? t('messages.noWorkspace')
            : t('market.item.downloadProject');
          const userTooltipText = t('market.item.downloadUser');
          const installedTooltipText = t('market.item.installedTooltip');

          return (
            <Card
              key={skill.installId}
              appearance="raised"
              padding="none"
              className={`alex-skills-config__market-item${isInstalled ? ' is-installed' : ''}`}
              data-alex-component="skills-config"
              data-alex-part="marketItem"
              data-alex-state={isInstalled ? 'installed' : undefined}
            >
              <CardBody className="alex-skills-config__market-item-body">
                <div className="alex-skills-config__market-item-main">
                  <div className="alex-skills-config__market-item-head">
                    <div className="alex-skills-config__market-item-name-wrap">
                      <div className="alex-skills-config__market-item-name">{skill.name}</div>
                      {isInstalled ? (
                        <span className="alex-skills-config__market-item-badge alex-skills-config__market-item-badge--installed">
                          <Icon name="check-circle" size="xs" />
                          {t('market.item.installed')}
                        </span>
                      ) : null}
                    </div>
                    <span className="alex-skills-config__market-item-installs">
                      <TrendingUp size={12} />
                      {t('market.item.installs', { count: skill.installs })}
                    </span>
                  </div>
                  <OverflowText as="div" lines={3} className="alex-skills-config__market-item-description">
                    {skill.description?.trim() || t('market.item.noDescription')}
                  </OverflowText>
                  <div className="alex-skills-config__market-item-meta">
                    {skill.source ? (
                      sourceLabel !== skill.source ? (
                        <Tooltip content={skill.source}>
                          <span className="alex-skills-config__market-item-chip alex-skills-config__market-item-source"><OverflowText>
                            {t('market.item.sourceLabel')}{sourceLabel}
                          </OverflowText></span>
                        </Tooltip>
                      ) : (
                        <span className="alex-skills-config__market-item-chip alex-skills-config__market-item-source"><OverflowText>
                          {t('market.item.sourceLabel')}{sourceLabel}
                        </OverflowText></span>
                      )
                    ) : null}
                  </div>
                </div>

                <div className="alex-skills-config__market-item-action">
                  {isInstalled ? (
                    <Tooltip content={installedTooltipText}>
                      <span>
                        <Button
                          className="alex-skills-config__market-action-button"
                          variant="fill"
                          size="sm"
                          disabled
                          leadingIcon={<Icon name="check-circle" size="sm" />}
                        >

                          {t('market.item.installed')}
                        </Button>
                      </span>
                    </Tooltip>
                  ) : (
                    <>
                      {!isRemote && (
                        <Tooltip content={projectTooltipText}>
                          <span>
                            <Button
                              className="alex-skills-config__market-action-button"
                              variant="primary"
                              size="sm"
                              onClick={() => handleDownload(skill, 'project')}
                              disabled={isDownloading || !hasWorkspace}
                              leadingIcon={<Icon name="arrow-down" size="sm" />}
                            >

                              {isDownloading ? t('market.item.downloading') : t('market.item.downloadProject')}
                            </Button>
                          </span>
                        </Tooltip>
                      )}
                      <Tooltip content={userTooltipText}>
                        <span>
                          <Button
                            className="alex-skills-config__market-action-button"
                            variant={isRemote ? 'primary' : 'outline'}
                            size="sm"
                            onClick={() => handleDownload(skill, 'user')}
                            disabled={isDownloading}
                            leadingIcon={<Icon name="arrow-down" size="sm" />}
                          >

                            {isDownloading ? t('market.item.downloading') : t('market.item.downloadUser')}
                          </Button>
                        </span>
                      </Tooltip>
                    </>
                  )}
                </div>
              </CardBody>
            </Card>
          );
        })}
      </div>
    );
  };

  const refreshExtra = (
    <Tooltip content={t('toolbar.refreshTooltip')}>
      <IconButton
        aria-label={t('toolbar.refreshTooltip')}
        size="sm"
        onClick={() => loadSkills(true)}
        icon={<Icon name="refresh" size="md" />}
      />
    </Tooltip>
  );

  const makeAddExtra = (level: SkillLevel) => (
    <>
      {level === 'user' && refreshExtra}
      <Tooltip content={t('toolbar.addTooltip')}>
        <IconButton
          aria-label={t('toolbar.addTooltip')}
          variant="primary"
          size="sm"
          onClick={() => { setFormLevel(level); setShowAddForm(true); }}
          disabled={level === 'project' && !hasWorkspace}
          icon={<Icon name="plus" size="md" />}
        />
      </Tooltip>
    </>
  );

  const installedMarketIds = useMemo(
    () => installedSkillMarketIds(skills),
    [skills]
  );

  const formatMarketSource = useCallback((source: string): string => {
    const raw = source.trim();
    if (!raw) return raw;

    const compact = raw
      .replace(/^https?:\/\//i, '')
      .replace(/^www\./i, '')
      .replace(/\/+$/, '');

    const parts = compact.split('/').filter(Boolean);
    if (parts.length === 0) return raw;
    if (parts.length === 1) return parts[0];

    if (parts[0].includes('.')) {
      return parts.slice(0, 2).join('/');
    }

    return parts.slice(0, 2).join('/');
  }, []);

  const displayMarketSkills = useMemo(() => {
    const entries = marketSkills.map((skill, index) => ({
      skill,
      index,
      installed: isSkillMarketItemInstalled(skill, installedMarketIds),
    }));

    entries.sort((a, b) => {
      if (a.installed !== b.installed) {
        return a.installed ? -1 : 1;
      }

      const installDelta = (b.skill.installs ?? 0) - (a.skill.installs ?? 0);
      if (installDelta !== 0) {
        return installDelta;
      }

      return a.index - b.index;
    });

    return entries.map((entry) => entry.skill);
  }, [marketSkills, installedMarketIds]);

  const handleMarketSearch = useCallback(() => {
    loadMarketSkills(marketKeyword);
  }, [loadMarketSkills, marketKeyword]);

  if (loading) {
    return (
      <ConfigPageLayout className="alex-skills-config" data-alex-component="skills-config" data-alex-part="root" data-alex-state="loading">
        <ConfigPageHeader title={t('title')} subtitle={t('subtitle')} />
        <ConfigPageContent data-alex-component="skills-config" data-alex-part="content">
          <div className="alex-collection-empty" data-alex-component="skills-config" data-alex-part="loading" data-alex-state="loading"><p>{t('list.loading')}</p></div>
        </ConfigPageContent>
      </ConfigPageLayout>
    );
  }

  if (error) {
    return (
      <ConfigPageLayout className="alex-skills-config" data-alex-component="skills-config" data-alex-part="root" data-alex-state="error">
        <ConfigPageHeader title={t('title')} subtitle={t('subtitle')} />
        <ConfigPageContent data-alex-component="skills-config" data-alex-part="content">
          <div className="alex-collection-empty" data-alex-component="skills-config" data-alex-part="error" data-alex-state="error"><p>{t('list.errorPrefix')}{error}</p></div>
        </ConfigPageContent>
      </ConfigPageLayout>
    );
  }

  const userSkills = skills.filter(s => s.level === 'user');
  const projectSkills = skills.filter(s => s.level === 'project');

  return (
    <ConfigPageLayout className="alex-skills-config" data-alex-component="skills-config" data-alex-part="root">
      <ConfigPageHeader title={t('title')} subtitle={t('subtitle')} />

      <ConfigPageContent data-alex-component="skills-config" data-alex-part="content">
        <ConfigPageSection
          title={t('market.title')}
          description={t('market.subtitle')}
          extra={(
            <Tooltip content={t('market.refreshTooltip')}>
              <IconButton
                aria-label={t('market.refreshTooltip')}
                size="sm"
                onClick={() => loadMarketSkills(marketKeyword)}
                icon={<Icon name="refresh" size="md" />}
              />
            </Tooltip>
          )}
        >
          <div className="alex-skills-config__market-toolbar" data-alex-component="skills-config" data-alex-part="marketToolbar">
            <SearchField
              className="alex-skills-config__market-search"
              placeholder={t('market.searchPlaceholder')}
              aria-label={t('market.searchPlaceholder')}
              leadingIcon={<Icon name="search" size="sm" aria-hidden />}
              value={marketKeyword}
              onValueChange={(value) => setMarketKeyword(value)}
              onSearch={handleMarketSearch}
              clearLabel={marketKeyword ? tShared('components:search.clear') : undefined}
              onClear={marketKeyword ? () => setMarketKeyword('') : undefined}
              size="sm"
            />
            <Button size="sm" variant="primary" onClick={handleMarketSearch}>
              {tShared('common:actions.search')}
            </Button>
          </div>
          {renderMarketList()}
        </ConfigPageSection>

        <ConfigPageSection
          title={t('filters.user')}
          description={t('section.user.description')}
          extra={makeAddExtra('user')}
        >
          {renderAddForm('user')}
          {userSkills.length === 0 && !(showAddForm && formLevel === 'user') ? (
            <div className="alex-collection-empty" data-alex-component="skills-config" data-alex-part="empty">
              <Button variant="outline" size="sm" onClick={() => { setFormLevel('user'); setShowAddForm(true); }} leadingIcon={<Icon name="plus" size="sm" />}>

                {t('toolbar.addTooltip')}
              </Button>
            </div>
          ) : userSkills.map(renderSkillRow)}
        </ConfigPageSection>

        <ConfigPageSection
          title={t('filters.project')}
          description={t('section.project.description')}
          extra={makeAddExtra('project')}
        >
          {renderAddForm('project')}
          {projectSkills.length === 0 && !(showAddForm && formLevel === 'project') ? (
            <div className="alex-collection-empty" data-alex-component="skills-config" data-alex-part="empty">
              {!hasWorkspace && <p>{t('messages.noWorkspace')}</p>}
              {hasWorkspace && (
                <Button variant="outline" size="sm" onClick={() => { setFormLevel('project'); setShowAddForm(true); }} leadingIcon={<Icon name="plus" size="sm" />}>

                  {t('toolbar.addTooltip')}
                </Button>
              )}
            </div>
          ) : projectSkills.map(renderSkillRow)}
        </ConfigPageSection>
      </ConfigPageContent>

      <ConfirmDialog
        open={deleteConfirm.show && !!deleteConfirm.skill}
        onOpenChange={() => setDeleteConfirm({ show: false, skill: null })}
        onConfirm={confirmDelete}
        title={t('deleteModal.title')}
        message={
          <>
            <p>{t('deleteModal.message', { name: deleteConfirm.skill?.name })}</p>
            <p style={{ marginTop: '8px', color: 'var(--alex-color-status-warning-content)' }}>{t('deleteModal.warning')}</p>
          </>
        }
        type="warning"
        confirmDanger
        confirmText={t('deleteModal.delete')}
        cancelText={t('deleteModal.cancel')}
      />
    </ConfigPageLayout>
  );
};

export default SkillsConfig;
