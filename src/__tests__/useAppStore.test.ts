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
      blockedApps: [],
      isExpanded: true,
      isLoading: false,
    });
  });

  describe('initial state', () => {
    test('has correct default values', () => {
      const state = useAppStore.getState();
      expect(state.isTracking).toBe(false);
      expect(state.currentApp).toBe(null);
      expect(state.stats).toBe(null);
      expect(state.sessions).toEqual([]);
      expect(state.isExpanded).toBe(true);
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
        .mockResolvedValueOnce(mockStatus);
      
      await useAppStore.getState().refreshStats();
      
      expect(useAppStore.getState().stats).toEqual(mockStats);
      expect(useAppStore.getState().currentApp).toBe('Chrome');
      expect(useAppStore.getState().currentWindowTitle).toBe('Google');
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
});
