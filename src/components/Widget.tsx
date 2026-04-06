import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
    currentAppSeconds,
    currentWindowTitle,
    stats, 
    sessions,
    isExpanded, 
    setExpanded,
    toggleTracking,
    refreshStats,
    refreshSessions,
    startTracking
  } = useAppStore();

  useEffect(() => {
    invoke('set_widget_expanded', { expanded: isExpanded }).catch((e) => {
      console.error('Failed to resize widget window:', e);
    });
  }, [isExpanded]);

  useEffect(() => {
    startTracking();
  }, []);

  useEffect(() => {
    if (isExpanded) {
      refreshSessions();
    }
  }, [isExpanded, refreshSessions]);

  useEffect(() => {
    const interval = setInterval(() => {
      if (isTracking) {
        refreshStats();
      }
    }, 3000);
    
    return () => clearInterval(interval);
  }, [isTracking, refreshStats]);

  useEffect(() => {
    const interval = setInterval(() => {
      if (isTracking) {
        refreshSessions();
      }
    }, 3000);

    return () => clearInterval(interval);
  }, [isTracking, refreshSessions]);

  const handleDragStart = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('button')) return;
    getCurrentWindow().startDragging().catch(console.error);
  };

  const handleExpand = () => {
    setExpanded(!isExpanded);
  };
  
  const currentSession = sessions.find(s => s.end_time === null && s.app_name === currentApp);
  const currentSessionSeconds = currentSession ? Math.floor(Date.now() / 1000) - currentSession.start_time : 0;
  const totalTime = currentSessionSeconds ?? 0;
  const topApp = stats?.most_used_app ?? 'No data yet';
  const recentSessions = sessions.slice(0, 8);
  const isChrome = (currentApp ?? '').toLowerCase().includes('chrome');
  const chromeTabTitle = isChrome ? (currentWindowTitle ?? null) : null;
  const isVisualStudio = (currentApp ?? '').toLowerCase().includes('visual studio');
  const visualStudioTabTitle = isVisualStudio ? (currentWindowTitle ?? null) : null;


  return (
    <div className={`widget ${isExpanded ? 'expanded' : ''}`}>
      <div 
        className="widget-header" 
        onMouseDown={handleDragStart}
      >
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
        </div>
      </div>

      {isExpanded ? (
        <div className="widget-content">
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
                <span className="tracking-indicator">●</span>
              ) : (
                <span className="idle">Idle</span>
              )}
              {currentApp || 'None'}
            </div>
            {chromeTabTitle && (
              <div className="activity-subvalue" title={chromeTabTitle}>
                {chromeTabTitle}
              </div>
            )}
            {visualStudioTabTitle && (
              <div className="activity-subvalue" title={visualStudioTabTitle}>
                {visualStudioTabTitle}
              </div>
            )}
          </div>

          <div className="sessions-list">
            <div className="sessions-header">Recent Sessions</div>
            {recentSessions.length > 0 ? (
              recentSessions.map((s) => (
                <div
                  key={`${s.id ?? 'no-id'}-${s.start_time}-${s.app_name}`}
                  className="session-item"
                >
                  <div className="session-left">
                    <span className="session-app">{s.app_name}</span>
                    {s.window_title && (
                      <span className="session-title" title={s.window_title}>
                        {s.window_title}
                      </span>
                    )}
                  </div>
                  <span className="session-time">{formatDuration(s.end_time ? s.duration_seconds : Math.floor(Date.now() / 1000) - s.start_time)}</span>
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
                    <div 
                      className="usage-fill" 
                      style={{ width: `${app.percentage}%` }}
                    />
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
            <span className="compact-app" title={chromeTabTitle ?? currentApp ?? ''}>
              {chromeTabTitle || currentApp || '-'}
            </span>
            <span className="compact-app" title={visualStudioTabTitle ?? currentApp ?? ''}>
              {visualStudioTabTitle || currentApp || '-'}
            </span>
          </div>
        </div>
      )}
    </div>
  );
}

export default Widget;
