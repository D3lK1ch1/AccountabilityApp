import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface AppSession {
  id: number | null;
  app_name: string;
  window_title: string | null;
  start_time: number;
  end_time: number | null;
  duration_seconds: number;
}

interface TabSession {
  id: number | null;
  source: string;
  tab_url: string;
  tab_title: string;
  start_time: number;
  end_time: number | null;
  duration_seconds: number;
}

interface UsageData {
  app_name: string;
  total_seconds: number;
  percentage: number;
}

interface DashboardStats {
  total_tracked_seconds: number;
  most_used_app: string | null;
  usage_by_app: UsageData[];
  sessions_count: number;
}

interface TrackerStatus {
  is_tracking: boolean;
  current_app: string | null;
  current_window_title: string | null;
}

interface BlockedApp {
  id: number | null;
  app_name: string;
  block_duration_minutes: number;
  enabled: boolean;
}

interface AppStore {
  isTracking: boolean;
  currentApp: string | null;
  currentWindowTitle: string | null;
  currentAppSeconds: number;
  stats: DashboardStats | null;
  sessions: AppSession[];
  tabSessions: TabSession[];
  blockedApps: BlockedApp[];
  isExpanded: boolean;
  isLoading: boolean;
  lastError: string | null;
  consentGiven: boolean | null;
  checkConsent: () => Promise<void>;
  startTracking: () => Promise<void>;
  stopTracking: () => Promise<void>;
  refreshStats: () => Promise<void>;
  refreshSessions: () => Promise<void>;
  refreshTabSessions: () => Promise<void>;
  toggleTracking: () => Promise<void>;
  setExpanded: (expanded: boolean) => void;
  clearAllSessions: () => Promise<void>;
  addBlockedApp: (appName: string, duration: number) => Promise<void>;
  removeBlockedApp: (appName: string) => Promise<void>;
  refreshBlockedApps: () => Promise<void>;
}

export const useAppStore = create<AppStore>((set, get) => ({
  isTracking: false,
  currentApp: null,
  currentWindowTitle: null,
  currentAppSeconds: 0,
  stats: null,
  sessions: [],
  tabSessions: [],
  blockedApps: [],
  isExpanded: true,
  isLoading: false,
  lastError: null,
  consentGiven: null,

  checkConsent: async () => {
    try {
      const consent = await invoke<string | null>('get_setting', { key: 'consent_given' }); 
      if  (consent === 'true') {
        set({ consentGiven: true });
      } else {
        set({ consentGiven: false });
      }
    } catch (e) {
      set({ lastError: 'Failed to check consent.' });
      console.error('Failed to check consent:', e);
    }
  },



  startTracking: async () => {
    try {
      await invoke('start_tracking');
      set({ isTracking: true, lastError: null });
      get().refreshStats();
      get().refreshSessions();
    } catch (e) {
      set({ lastError: 'Failed to start tracking.' });
      console.error('Failed to start tracking:', e);
    }
  },

  stopTracking: async () => {
    try {
      await invoke('stop_tracking');
      set({ isTracking: false, lastError: null });
    } catch (e) {
      set({ lastError: 'Failed to stop tracking.' });
      console.error('Failed to stop tracking:', e);
    }
  },

  refreshStats: async () => {
    try {
      const stats = await invoke<DashboardStats>('get_dashboard_stats');
      const status = await invoke<TrackerStatus>('get_tracker_status');
      const time = status.current_app
        ? await invoke<number>('get_tracked_time_per_app', { appName: status.current_app })
        : 0;
      set({ 
        stats, 
        currentApp: status.current_app,
        currentAppSeconds: status.current_app ? time : 0,
        currentWindowTitle: status.current_window_title,
        lastError: null,
      });
    } catch (e) {
      set({ lastError: 'Failed to refresh stats.' });
      console.error('Failed to refresh stats:', e);
    }
  },

  refreshSessions: async () => {
    try {
      const sessions = await invoke<AppSession[]>('get_sessions_today');
      set({ sessions, lastError: null });
    } catch (e) {
      set({ lastError: 'Failed to refresh sessions.' });
      console.error('Failed to refresh sessions:', e);
    }
  },

  refreshTabSessions: async () => {
    try {
      const tabSessions = await invoke<TabSession[]>('get_tab_sessions_today');
      set({ tabSessions, lastError: null });
    } catch (e) {
      set({ lastError: 'Failed to refresh tab sessions.' });
      console.error('Failed to refresh tab sessions:', e);
    }
  },

  toggleTracking: async () => {
    const { isTracking, startTracking, stopTracking } = get();
    if (isTracking) {
      await stopTracking();
    } else {
      await startTracking();
    }
  },

  setExpanded: (expanded) => {
    set({ isExpanded: expanded });
    if (expanded) {
      get().refreshStats();
      get().refreshSessions();
    }
  },

  clearAllSessions: async () => {
    try {
      await invoke('clear_all_sessions');
      get().refreshStats();
      get().refreshSessions();
      get().refreshTabSessions();
    } catch (e) {
      set({ lastError: 'Failed to clear session data.' });
      console.error('Failed to clear all sessions:', e);
    }
  },

  addBlockedApp: async (appName, duration) => {
    try {
      await invoke('add_blocked_app', { appName, blockDurationMinutes: duration });
      get().refreshBlockedApps();
    } catch (e) {
      set({ lastError: 'Failed to add blocked app.' });
      console.error('Failed to add blocked app:', e);
    }
  },

  removeBlockedApp: async (appName) => {
    try {
      await invoke('remove_blocked_app', { appName });
      get().refreshBlockedApps();
    } catch (e) {
      set({ lastError: 'Failed to remove blocked app.' });
      console.error('Failed to remove blocked app:', e);
    }
  },

  refreshBlockedApps: async () => {
    try {
      const blockedApps = await invoke<BlockedApp[]>('get_blocked_apps');
      set({ blockedApps, lastError: null });
    } catch (e) {
      set({ lastError: 'Failed to refresh blocked apps.' });
      console.error('Failed to refresh blocked apps:', e);
    }
  },
}));
