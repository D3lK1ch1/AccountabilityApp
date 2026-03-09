import { useEffect } from 'react';
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
    stats, 
    isExpanded, 
    setExpanded,
    toggleTracking,
    refreshStats,
    startTracking
  } = useAppStore();

  useEffect(() => {
    startTracking();
  }, []);

  useEffect(() => {
    const interval = setInterval(() => {
      if (isTracking) {
        refreshStats();
      }
    }, 3000);
    
    return () => clearInterval(interval);
  }, [isTracking, refreshStats]);

  const handleDragStart = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('button')) return;
    getCurrentWindow().startDragging().catch(console.error);
  };

  const handleExpand = () => {
    setExpanded(!isExpanded);
  };

  const totalTime = stats?.total_tracked_seconds ?? 0;
  const topApp = stats?.most_used_app ?? 'No data yet';

  return (
    <div className={`widget ${isExpanded ? 'expanded' : ''}`}>
      <div 
        className="widget-header" 
        onMouseDown={handleDragStart}
      >
        <div className="header-left">
          <span className="app-icon">📊</span>
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
              <div className="stat-label">Total Today</div>
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
            <span className="compact-app">{currentApp || '-'}</span>
          </div>
        </div>
      )}
    </div>
  );
}

export default Widget;
