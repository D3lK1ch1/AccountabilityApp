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
  stats: DashboardStats | null;
  sessions: AppSession[];
  blockedApps: BlockedApp[];
  isExpanded: boolean;
  isLoading: boolean;
  
  startTracking: () => Promise<void>;
  stopTracking: () => Promise<void>;
  refreshStats: () => Promise<void>;
  refreshSessions: () => Promise<void>;
  toggleTracking: () => Promise<void>;
  setExpanded: (expanded: boolean) => void;
  addBlockedApp: (appName: string, duration: number) => Promise<void>;
  removeBlockedApp: (appName: string) => Promise<void>;
  refreshBlockedApps: () => Promise<void>;
}

export const useAppStore = create<AppStore>((set, get) => ({
  isTracking: false,
  currentApp: null,
  currentWindowTitle: null,
  stats: null,
  sessions: [],
  blockedApps: [],
  isExpanded: true,
  isLoading: false,

  startTracking: async () => {
    try {
      await invoke('start_tracking');
      set({ isTracking: true });
      get().refreshStats();
    } catch (e) {
      console.error('Failed to start tracking:', e);
    }
  },

  stopTracking: async () => {
    try {
      await invoke('stop_tracking');
      set({ isTracking: false });
    } catch (e) {
      console.error('Failed to stop tracking:', e);
    }
  },

  refreshStats: async () => {
    try {
      const stats = await invoke<DashboardStats>('get_dashboard_stats');
      const status = await invoke<TrackerStatus>('get_tracker_status');
      set({ 
        stats, 
        currentApp: status.current_app, 
        currentWindowTitle: status.current_window_title 
      });
    } catch (e) {
      console.error('Failed to refresh stats:', e);
    }
  },

  refreshSessions: async () => {
    try {
      const sessions = await invoke<AppSession[]>('get_sessions_today');
      set({ sessions });
    } catch (e) {
      console.error('Failed to refresh sessions:', e);
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

  addBlockedApp: async (appName, duration) => {
    try {
      await invoke('add_blocked_app', { appName, blockDurationMinutes: duration });
      get().refreshBlockedApps();
    } catch (e) {
      console.error('Failed to add blocked app:', e);
    }
  },

  removeBlockedApp: async (appName) => {
    try {
      await invoke('remove_blocked_app', { appName });
      get().refreshBlockedApps();
    } catch (e) {
      console.error('Failed to remove blocked app:', e);
    }
  },

  refreshBlockedApps: async () => {
    try {
      const blockedApps = await invoke<BlockedApp[]>('get_blocked_apps');
      set({ blockedApps });
    } catch (e) {
      console.error('Failed to refresh blocked apps:', e);
    }
  },
}));
