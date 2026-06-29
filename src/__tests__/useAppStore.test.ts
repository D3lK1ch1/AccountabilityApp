import { describe, test, expect, vi, beforeEach } from 'vitest';
import { useAppStore } from '../store/useAppStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe('useAppStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      isTracking: false,
      currentApp: null,
      currentWindowTitle: null,
      stats: null,
      sessions: [],
      tabSessions: [],
      blockedApps: [],
      blockCategories: [],
      categoryUsage: [],
      isExpanded: true,
      isLoading: false,
      lastError: null,
    });
  });

  describe('initial state', () => {
    test('has correct default values', () => {
      const state = useAppStore.getState();
      expect(state.isTracking).toBe(false);
      expect(state.currentApp).toBe(null);
      expect(state.stats).toBe(null);
      expect(state.sessions).toEqual([]);
      expect(state.tabSessions).toEqual([]);
      expect(state.blockCategories).toEqual([]);
      expect(state.categoryUsage).toEqual([]);
      expect(state.isExpanded).toBe(true);
      expect(state.lastError).toBe(null);
    });
  });

  describe('setExpanded', () => {
    test('sets isExpanded to false', () => {
      useAppStore.getState().setExpanded(false);
      expect(useAppStore.getState().isExpanded).toBe(false);
    });

    test('sets isExpanded to true', () => {
      useAppStore.setState({ isExpanded: false });
      useAppStore.getState().setExpanded(true);
      expect(useAppStore.getState().isExpanded).toBe(true);
    });
  });

  describe('startTracking', () => {
    test('sets isTracking to true on success', async () => {
      await useAppStore.getState().startTracking();
      expect(useAppStore.getState().isTracking).toBe(true);
    });
  });

  describe('stopTracking', () => {
    test('sets isTracking to false', async () => {
      useAppStore.setState({ isTracking: true });
      await useAppStore.getState().stopTracking();
      expect(useAppStore.getState().isTracking).toBe(false);
    });
  });

  describe('toggleTracking', () => {
    test('toggles from false to true', async () => {
      await useAppStore.getState().toggleTracking();
      expect(useAppStore.getState().isTracking).toBe(true);
    });

    test('toggles from true to false', async () => {
      useAppStore.setState({ isTracking: true });
      await useAppStore.getState().toggleTracking();
      expect(useAppStore.getState().isTracking).toBe(false);
    });
  });

  describe('refreshStats', () => {
    test('updates stats from API response', async () => {
      const mockStats = {
        total_tracked_seconds: 3600,
        most_used_app: 'Chrome',
        usage_by_app: [],
        sessions_count: 5,
      };
      const mockStatus = {
        is_tracking: true,
        current_app: 'Chrome',
        current_window_title: 'Google',
      };
      
      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>)
        .mockResolvedValueOnce(mockStats)
        .mockResolvedValueOnce(mockStatus)
        .mockResolvedValueOnce(3600);
      
      await useAppStore.getState().refreshStats();
      
      expect(useAppStore.getState().stats).toEqual(mockStats);
      expect(useAppStore.getState().currentApp).toBe('Chrome');
      expect(useAppStore.getState().currentWindowTitle).toBe('Google');
      expect(useAppStore.getState().currentAppSeconds).toBe(3600);
    });

    test('does not request per-app time when no app is active', async () => {
      const mockStats = {
        total_tracked_seconds: 0,
        most_used_app: null,
        usage_by_app: [],
        sessions_count: 0,
      };
      const mockStatus = {
        is_tracking: true,
        current_app: null,
        current_window_title: null,
      };

      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>)
        .mockResolvedValueOnce(mockStats)
        .mockResolvedValueOnce(mockStatus);

      await useAppStore.getState().refreshStats();

      expect(invoke).not.toHaveBeenCalledWith('get_tracked_time_per_app', expect.anything());
      expect(useAppStore.getState().currentApp).toBe(null);
      expect(useAppStore.getState().currentAppSeconds).toBe(0);
    });
  });

  describe('refreshSessions', () => {
    test('updates sessions from API response', async () => {
      const mockSessions = [
        {
          id: 1,
          app_name: 'Chrome',
          window_title: 'Google',
          start_time: Date.now(),
          end_time: null,
          duration_seconds: 120,
        },
      ];
      
      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(mockSessions);
      
      await useAppStore.getState().refreshSessions();
      
      expect(useAppStore.getState().sessions).toEqual(mockSessions);
    });
  });

  describe('blocked apps', () => {
    test('refreshBlockedApps updates blockedApps from API', async () => {
      const mockBlockedApps = [
        { id: 1, app_name: 'Discord', block_duration_minutes: 10, enabled: true },
      ];
      
      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(mockBlockedApps);
      
      await useAppStore.getState().refreshBlockedApps();
      
      expect(useAppStore.getState().blockedApps).toEqual(mockBlockedApps);
    });
  });

  describe('tab sessions', () => {
    test('refreshTabSessions updates tabSessions from API', async () => {
      const mockTabSessions = [
        {
          id: 1,
          source: 'chrome',
          tab_url: 'https://example.com',
          tab_title: 'Example',
          start_time: Date.now(),
          end_time: null,
          duration_seconds: 0,
        },
      ];

      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(mockTabSessions);

      await useAppStore.getState().refreshTabSessions();

      expect(useAppStore.getState().tabSessions).toEqual(mockTabSessions);
    });

    test('refreshTabSessions leaves tabSessions unchanged on error', async () => {
      useAppStore.setState({ tabSessions: [] });
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('backend error'));

      await expect(useAppStore.getState().refreshTabSessions()).resolves.toBeUndefined();

      expect(useAppStore.getState().tabSessions).toEqual([]);
      consoleSpy.mockRestore();
    });
  });

  describe('block categories', () => {
    test('refreshBlockCategories updates blockCategories from API', async () => {
      const categories = [
        {
          id: 1,
          name: 'Social Media',
          daily_limit_minutes: 60,
          enabled: true,
          manual_block_paused: false,
          domain_keywords: ['instagram.com'],
          app_keywords: ['discord'],
          display_order: 0,
        },
      ];

      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(categories);

      await useAppStore.getState().refreshBlockCategories();

      expect(useAppStore.getState().blockCategories).toEqual(categories);
    });

    test('refreshCategoryUsage updates categoryUsage from API', async () => {
      const usage = [
        {
          category_id: 1,
          category_name: 'Social Media',
          used_seconds: 3600,
          limit_seconds: 3600,
          limit_exceeded: true,
          enabled: true,
          manual_block_paused: false,
        },
      ];

      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(usage);

      await useAppStore.getState().refreshCategoryUsage();

      expect(useAppStore.getState().categoryUsage).toEqual(usage);
    });

    test('saveBlockCategory calls upsert and refreshes category state', async () => {
      const category = {
        id: 1,
        name: 'Games',
        daily_limit_minutes: 90,
        enabled: true,
        manual_block_paused: false,
        domain_keywords: ['steampowered.com'],
        app_keywords: ['steam'],
        display_order: 1,
      };

      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>)
        .mockResolvedValueOnce(1)
        .mockResolvedValueOnce([category])
        .mockResolvedValueOnce([]);

      await useAppStore.getState().saveBlockCategory(category);

      expect(invoke).toHaveBeenCalledWith('upsert_block_category', { category });
      expect(useAppStore.getState().blockCategories).toEqual([category]);
    });

    test('setBlockCategoryPaused calls backend with categoryId', async () => {
      const { invoke } = await import('@tauri-apps/api/core');
      (invoke as ReturnType<typeof vi.fn>)
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce([]);

      await useAppStore.getState().setBlockCategoryPaused(2, true);

      expect(invoke).toHaveBeenCalledWith('set_block_category_paused', {
        categoryId: 2,
        paused: true,
      });
    });
  });
});
