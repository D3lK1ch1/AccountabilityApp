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

interface BlockCategory {
  id: number | null;
  name: string;
  daily_limit_minutes: number;
  enabled: boolean;
  manual_block_paused: boolean;
  domain_keywords: string[];
  app_keywords: string[];
  display_order: number;
}

interface CategoryUsage {
  category_id: number;
  category_name: string;
  used_seconds: number;
  limit_seconds: number;
  limit_exceeded: boolean;
  enabled: boolean;
  manual_block_paused: boolean;
}

interface AppStore {
  isTracking: boolean;
  currentApp: string | null;
  currentWindowTitle: string | null;
  currentAppSeconds: number;
  stats: DashboardStats | null;
  sessions: AppSession[];
  blockedApps: BlockedApp[];
  blockCategories: BlockCategory[];
  categoryUsage: CategoryUsage[];
  isExpanded: boolean;
  isLoading: boolean;
  lastError: string | null;
  consentGiven: boolean | null;
  checkConsent: () => Promise<void>;
  startTracking: () => Promise<void>;
  stopTracking: () => Promise<void>;
  refreshStats: () => Promise<void>;
  refreshSessions: () => Promise<void>;
  toggleTracking: () => Promise<void>;
  setExpanded: (expanded: boolean) => void;
  clearAllSessions: () => Promise<void>;
  addBlockedApp: (appName: string, duration: number) => Promise<void>;
  removeBlockedApp: (appName: string) => Promise<void>;
  refreshBlockedApps: () => Promise<void>;
  refreshBlockCategories: () => Promise<void>;
  refreshCategoryUsage: () => Promise<void>;
  saveBlockCategory: (category: BlockCategory) => Promise<void>;
  setBlockCategoryEnabled: (categoryId: number, enabled: boolean) => Promise<void>;
  setBlockCategoryPaused: (categoryId: number, paused: boolean) => Promise<void>;
}

export const useAppStore = create<AppStore>((set, get) => {
  // Every store action needs the same invoke -> on failure, set lastError + log
  // shape. Centralizing it means a change to that shape (e.g. reporting to telemetry)
  // happens in one place instead of 15.
  const runAction = async (errorMessage: string, action: () => Promise<void>) => {
    try {
      await action();
    } catch (e) {
      set({ lastError: errorMessage });
      console.error(errorMessage, e);
    }
  };

  return {
    isTracking: false,
    currentApp: null,
    currentWindowTitle: null,
    currentAppSeconds: 0,
    stats: null,
    sessions: [],
    blockedApps: [],
    blockCategories: [],
    categoryUsage: [],
    isExpanded: true,
    isLoading: false,
    lastError: null,
    consentGiven: null,

    checkConsent: async () => {
      await runAction('Failed to check consent.', async () => {
        const consent = await invoke<string | null>('get_setting', { key: 'consent_given' });
        set({ consentGiven: consent === 'true' });
      });
    },

    startTracking: async () => {
      await runAction('Failed to start tracking.', async () => {
        await invoke('start_tracking');
        set({ isTracking: true, lastError: null });
        get().refreshStats();
        get().refreshSessions();
      });
    },

    stopTracking: async () => {
      await runAction('Failed to stop tracking.', async () => {
        await invoke('stop_tracking');
        set({ isTracking: false, lastError: null });
      });
    },

    refreshStats: async () => {
      await runAction('Failed to refresh stats.', async () => {
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
      });
    },

    refreshSessions: async () => {
      await runAction('Failed to refresh sessions.', async () => {
        const sessions = await invoke<AppSession[]>('get_sessions_today');
        set({ sessions, lastError: null });
      });
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
      await runAction('Failed to clear session data.', async () => {
        await invoke('clear_all_sessions');
        get().refreshStats();
        get().refreshSessions();
      });
    },

    addBlockedApp: async (appName, duration) => {
      await runAction('Failed to add blocked app.', async () => {
        await invoke('add_blocked_app', { appName, blockDurationMinutes: duration });
        get().refreshBlockedApps();
      });
    },

    removeBlockedApp: async (appName) => {
      await runAction('Failed to remove blocked app.', async () => {
        await invoke('remove_blocked_app', { appName });
        get().refreshBlockedApps();
      });
    },

    refreshBlockedApps: async () => {
      await runAction('Failed to refresh blocked apps.', async () => {
        const blockedApps = await invoke<BlockedApp[]>('get_blocked_apps');
        set({ blockedApps, lastError: null });
      });
    },

    refreshBlockCategories: async () => {
      await runAction('Failed to refresh block categories.', async () => {
        const blockCategories = await invoke<BlockCategory[]>('get_block_categories');
        set({ blockCategories, lastError: null });
      });
    },

    refreshCategoryUsage: async () => {
      await runAction('Failed to refresh category usage.', async () => {
        const categoryUsage = await invoke<CategoryUsage[]>('get_category_usage_today');
        set({ categoryUsage, lastError: null });
      });
    },

    saveBlockCategory: async (category) => {
      await runAction('Failed to save block category.', async () => {
        await invoke('upsert_block_category', { category });
        await get().refreshBlockCategories();
        await get().refreshCategoryUsage();
      });
    },

    setBlockCategoryEnabled: async (categoryId, enabled) => {
      await runAction('Failed to update block category.', async () => {
        await invoke('set_block_category_enabled', { categoryId, enabled });
        await get().refreshBlockCategories();
        await get().refreshCategoryUsage();
      });
    },

    setBlockCategoryPaused: async (categoryId, paused) => {
      await runAction('Failed to update category pause.', async () => {
        await invoke('set_block_category_paused', { categoryId, paused });
        await get().refreshBlockCategories();
        await get().refreshCategoryUsage();
      });
    },
  };
});
