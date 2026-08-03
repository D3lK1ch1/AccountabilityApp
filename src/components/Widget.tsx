import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAppStore } from '../store/useAppStore';
import './Widget.css';

function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${secs}s`;
  }
  return `${secs}s`;
}

export function Widget() {
  const {
    isTracking,
    currentApp,
    currentWindowTitle,
    stats,
    sessions,
    blockCategories,
    categoryUsage,
    isExpanded,
    lastError,
    setExpanded,
    toggleTracking,
    refreshStats,
    refreshSessions,
    refreshBlockCategories,
    refreshCategoryUsage,
    startTracking,
    stopTracking,
    clearAllSessions,
    saveBlockCategory,
    setBlockCategoryEnabled,
    setBlockCategoryPaused,
  } = useAppStore();

  const [show, setShow] = useState(false);
  const [keywordDraft, setKeywordDraft] = useState<Record<string, { domain: string; app: string }>>({});
  const [saveNotice, setSaveNotice] = useState<string | null>(null);

  useEffect(() => {
    invoke('set_widget_expanded', { expanded: isExpanded }).catch((e) => {
      console.error('Failed to resize widget window:', e);
    });
  }, [isExpanded]);

  useEffect(() => {
    startTracking();
    refreshBlockCategories();
    refreshCategoryUsage();
  }, [startTracking, refreshBlockCategories, refreshCategoryUsage]);

  useEffect(() => {
    if (isExpanded) {
      refreshSessions();
    }
  }, [isExpanded, refreshSessions]);

  useEffect(() => {
    const interval = setInterval(() => {
      if (isTracking) {
        refreshStats();
        refreshCategoryUsage();
      }
    }, 3000);

    return () => clearInterval(interval);
  }, [isTracking, refreshStats, refreshCategoryUsage]);

  useEffect(() => {
    const interval = setInterval(() => {
      if (isTracking) {
        refreshSessions();
      }
    }, 3000);

    return () => clearInterval(interval);
  }, [isTracking, refreshSessions]);

  const downloadSessionReport = async () => {
    try {
      const [rawNumber, { markdown, generated_at }] = await Promise.all([
        invoke<string | null>('get_setting', { key: 'next_session_report_number' }),
        invoke<{ markdown: string; generated_at: number }>('generate_session_report'),
      ]);
      const n = rawNumber ? parseInt(rawNumber, 10) : 1;

      const generatedDate = new Date(generated_at * 1000);
      const pad = (v: number) => String(v).padStart(2, '0');
      const dateStr = `${generatedDate.getFullYear()}-${pad(generatedDate.getMonth() + 1)}-${pad(generatedDate.getDate())}`;
      const timeStr = `${pad(generatedDate.getHours())}${pad(generatedDate.getMinutes())}`;
      const filename = `session_${String(n).padStart(2, '0')}_${dateStr}_${timeStr}.md`;

      const path = await save({
        defaultPath: filename,
        filters: [{ name: 'Markdown', extensions: ['md'] }],
      });
      if (!path) {
        console.log('downloadSessionReport: save dialog was cancelled');
        return;
      }

      await invoke('save_text_file', { path, contents: markdown });

      // Only advance the checkpoint and counter after a confirmed save —
      // cancelling must not skip data out of the next report.
      await invoke('set_setting', { key: 'last_report_generated_at', value: String(generated_at) });
      await invoke('set_setting', { key: 'next_session_report_number', value: String(n + 1) });

      setSaveNotice(`Saved ${filename}`);
      setTimeout(() => setSaveNotice(null), 4000);
    } catch (err) {
      console.error('downloadSessionReport failed:', err);
    }
  };

  const handleDragStart = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('button')) return;
    getCurrentWindow().startDragging().catch(console.error);
  };

  const handleExpand = () => {
    setExpanded(!isExpanded);
  };

  const currentSession = sessions.find((s) => s.end_time === null && s.app_name === currentApp);
  const currentSessionSeconds = currentSession ? Math.floor(Date.now() / 1000) - currentSession.start_time : 0;
  const totalTime = currentSessionSeconds ?? 0;
  const topApp = stats?.most_used_app ?? 'No data yet';
  const recentSessions = sessions.slice(0, 8);
  const appName = (currentApp ?? '').toLowerCase();
  const showWindowTitle = appName.includes('chrome') || appName.includes('visual studio');
  const detailTitle = showWindowTitle ? currentWindowTitle : null;
  const usageByCategoryId = new Map(categoryUsage.map((usage) => [usage.category_id, usage]));

  const handleLimitChange = (categoryId: number | null, value: string) => {
    const category = blockCategories.find((item) => item.id === categoryId);
    if (!category) return;
    const daily_limit_minutes = Math.max(1, Number.parseInt(value, 10) || 1);
    saveBlockCategory({ ...category, daily_limit_minutes });
  };

  type BlockCategory = (typeof blockCategories)[number];

  const draftKey = (category: BlockCategory) => String(category.id ?? category.name);

  const getDraftValue = (category: BlockCategory, field: 'domain' | 'app') => {
    const key = draftKey(category);
    if (keywordDraft[key]) return keywordDraft[key][field];
    return field === 'domain'
      ? category.domain_keywords.join(', ')
      : category.app_keywords.join(', ');
  };

  const handleKeywordChange = (category: BlockCategory, field: 'domain' | 'app', value: string) => {
    const key = draftKey(category);
    setKeywordDraft((prev) => ({
      ...prev,
      [key]: {
        domain: field === 'domain' ? value : (prev[key]?.domain ?? category.domain_keywords.join(', ')),
        app: field === 'app' ? value : (prev[key]?.app ?? category.app_keywords.join(', ')),
      },
    }));
  };

  const handleKeywordBlur = (category: BlockCategory, field: 'domain_keywords' | 'app_keywords', value: string) => {
    const keywords = value.split(',').map((k) => k.trim()).filter(Boolean);
    saveBlockCategory({ ...category, [field]: keywords });
    const key = draftKey(category);
    setKeywordDraft((prev) => {
      const next = { ...prev };
      delete next[key];
      return next;
    });
  };

  return (
    <div className={`widget ${isExpanded ? 'expanded' : ''}`}>
      <div className="widget-header" onMouseDown={handleDragStart}>
        <div className="header-left">
          <span className="title">Accountability</span>
        </div>
        <div className="header-right">
          <button
            className={`tracking-btn ${isTracking ? 'active' : ''}`}
            onClick={(e) => { e.stopPropagation(); toggleTracking(); }}
            title={isTracking ? 'Stop tracking' : 'Start tracking'}
          >
            {isTracking ? '⏹' : '▶'}
          </button>
          <button
            className="expand-btn"
            onClick={(e) => { e.stopPropagation(); handleExpand(); }}
            title={isExpanded ? 'Collapse' : 'Expand'}
          >
            {isExpanded ? '▼' : '▲'}
          </button>
          <button
            className="quit-btn"
            onClick={(e) => {
              e.stopPropagation();
              stopTracking();
              invoke('quit_app');
            }}
            title="Quit app"
          >
            ✖
          </button>
          <button
            className="clear-btn"
            onClick={(e) => {
              e.stopPropagation();
              setShow(true);
            }}
            title="Clear all data"
          >
            🗑
          </button>
          <button
            className="clear-btn"
            onClick={(e) => { e.stopPropagation(); downloadSessionReport(); }}
            title="Download session report"
          >
            <svg viewBox="0 0 512 512" width="1em" height="1em" fill="currentColor" aria-hidden="true">
              <path d="M256 0c-26.5 0-48 21.5-48 48v214.1l-73.4-73.4c-18.7-18.7-49.1-18.7-67.9 0s-18.7 49.1 0 67.9L222.1 412c18.7 18.7 49.1 18.7 67.9 0L446.4 256.6c18.7-18.7 18.7-49.1 0-67.9s-49.1-18.7-67.9 0L304 262.1V48c0-26.5-21.5-48-48-48zM64 352c-35.3 0-64 28.7-64 64v32c0 35.3 28.7 64 64 64h384c35.3 0 64-28.7 64-64v-32c0-35.3-28.7-64-64-64H346.5l-45.3 45.3c-25 25-65.5 25-90.5 0L165.5 352H64z" />
            </svg>
          </button>
        </div>
      </div>

      {show && (
        <div className="confirmation-overlay">
          <div className="confirmation-dialog">
            <p>Are you sure you want to clear all data? This action cannot be undone.</p>
            <div className="confirmation-buttons">
              <button
                className="confirm-btn"
                onClick={() => {
                  clearAllSessions();
                  setShow(false);
                }}
              >
                Yes
              </button>
              <button className="cancel-btn" onClick={() => setShow(false)}>
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {isExpanded ? (
        <div className="widget-content">
          {lastError && (
            <div className="error-banner" role="alert">
              {lastError}
            </div>
          )}
          {saveNotice && (
            <div className="save-banner" role="status">
              {saveNotice}
            </div>
          )}
          <div className="stats-grid">
            <div className="stat-card">
              <div className="stat-label">Current App</div>
              <div className="stat-value">{formatDuration(totalTime)}</div>
            </div>
            <div className="stat-card">
              <div className="stat-label">Most Used</div>
              <div className="stat-value app-name">{topApp}</div>
            </div>
          </div>

          <div className="current-activity">
            <div className="activity-label">Current Activity</div>
            <div className="activity-value">
              {currentApp ? (
                <span className="tracking-indicator">On</span>
              ) : (
                <span className="idle">Idle</span>
              )}
              {currentApp || 'None'}
            </div>
            {detailTitle && (
              <div className="activity-subvalue" title={detailTitle}>
                {detailTitle}
              </div>
            )}
          </div>

          <div className="block-categories">
            <div className="sessions-header">Category Limits</div>
            {blockCategories.length > 0 ? (
              blockCategories.map((category) => {
                const usage = category.id === null ? undefined : usageByCategoryId.get(category.id);
                const usedSeconds = usage?.used_seconds ?? 0;
                const limitSeconds = usage?.limit_seconds ?? category.daily_limit_minutes * 60;
                const percent = limitSeconds > 0 ? Math.min(100, (usedSeconds / limitSeconds) * 100) : 0;

                return (
                  <div key={category.id ?? category.name} className="category-item">
                    <div className="category-top">
                      <span className="category-name">{category.name}</span>
                      <span className={`category-status ${usage?.limit_exceeded ? 'over' : ''}`}>
                        {formatDuration(usedSeconds)} / {formatDuration(limitSeconds)}
                      </span>
                    </div>
                    <div className="category-bar">
                      <div className="category-fill" style={{ width: `${percent}%` }} />
                    </div>
                    <div className="category-controls">
                      <label>
                        Limit
                        <input
                          type="number"
                          min="1"
                          value={category.daily_limit_minutes}
                          onChange={(e) => handleLimitChange(category.id, e.target.value)}
                        />
                      </label>
                      <label>
                        Enabled
                        <input
                          type="checkbox"
                          checked={category.enabled}
                          onChange={(e) => {
                            if (category.id !== null) {
                              setBlockCategoryEnabled(category.id, e.target.checked);
                            }
                          }}
                        />
                      </label>
                      <label>
                        Paused
                        <input
                          type="checkbox"
                          checked={category.manual_block_paused}
                          onChange={(e) => {
                            if (category.id !== null) {
                              setBlockCategoryPaused(category.id, e.target.checked);
                            }
                          }}
                        />
                      </label>
                    </div>
                    <input
                      className="keyword-input"
                      value={getDraftValue(category, 'domain')}
                      onChange={(e) => handleKeywordChange(category, 'domain', e.target.value)}
                      onBlur={(e) => handleKeywordBlur(category, 'domain_keywords', e.target.value)}
                      aria-label={`${category.name} domains`}
                    />
                    <input
                      className="keyword-input"
                      value={getDraftValue(category, 'app')}
                      onChange={(e) => handleKeywordChange(category, 'app', e.target.value)}
                      onBlur={(e) => handleKeywordBlur(category, 'app_keywords', e.target.value)}
                      aria-label={`${category.name} apps`}
                    />
                  </div>
                );
              })
            ) : (
              <div className="sessions-empty">No category limits yet</div>
            )}
          </div>

          <div className="sessions-list">
            <div className="sessions-header">Recent Sessions</div>
            {recentSessions.length > 0 ? (
              recentSessions.map((s) => (
                <div key={`${s.id ?? 'no-id'}-${s.start_time}-${s.app_name}`} className="session-item">
                  <div className="session-left">
                    <span className="session-app">{s.app_name}</span>
                    {s.window_title && (
                      <span className="session-title" title={s.window_title}>
                        {s.window_title}
                      </span>
                    )}
                  </div>
                  <span className="session-time">
                    {formatDuration(s.end_time ? s.duration_seconds : Math.floor(Date.now() / 1000) - s.start_time)}
                  </span>
                </div>
              ))
            ) : (
              <div className="sessions-empty">No sessions yet</div>
            )}
          </div>

          {stats?.usage_by_app && stats.usage_by_app.length > 0 && (
            <div className="usage-list">
              <div className="usage-header">App Usage</div>
              {stats.usage_by_app.slice(0, 5).map((app) => (
                <div key={app.app_name} className="usage-item">
                  <span className="app-name">{app.app_name}</span>
                  <span className="app-time">{formatDuration(app.total_seconds)}</span>
                  <div className="usage-bar">
                    <div className="usage-fill" style={{ width: `${app.percentage}%` }} />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      ) : (
        <div className="widget-compact">
          <div className="compact-stats">
            <span className={`status-dot ${isTracking ? 'active' : ''}`}></span>
            <span className="compact-time">{formatDuration(totalTime)}</span>
            <span className="compact-app" title={detailTitle ?? currentApp ?? ''}>
              {detailTitle || currentApp || '-'}
            </span>
          </div>
        </div>
      )}
    </div>
  );
}

export default Widget;
