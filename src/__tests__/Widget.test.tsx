import { render, screen, fireEvent } from '@testing-library/react';
import { describe, test, expect, vi, beforeEach } from 'vitest';
import { Widget } from '../components/Widget';
import * as useAppStoreModule from '../store/useAppStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn().mockReturnValue({
    startDragging: vi.fn().mockResolvedValue(undefined),
  }),
}));

vi.mock('../store/useAppStore');

const mockStore = {
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
  startTracking: vi.fn(),
  stopTracking: vi.fn(),
  refreshStats: vi.fn(),
  refreshSessions: vi.fn(),
  refreshTabSessions: vi.fn(),
  toggleTracking: vi.fn(),
  setExpanded: vi.fn(),
  clearAllSessions: vi.fn(),
  addBlockedApp: vi.fn(),
  removeBlockedApp: vi.fn(),
  refreshBlockedApps: vi.fn(),
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.spyOn(useAppStoreModule, 'useAppStore').mockImplementation(() => mockStore);
});

describe('Widget', () => {
  test('renders in expanded mode by default', () => {
    render(<Widget />);

    expect(screen.getByText('Accountability')).toBeInTheDocument();
    expect(screen.getByText('Current App')).toBeInTheDocument();
    expect(screen.getByText('Most Used')).toBeInTheDocument();
    expect(screen.getByText('Current Activity')).toBeInTheDocument();
  });

  test('renders tracking button with start label when not tracking', () => {
    render(<Widget />);

    const trackingBtn = screen.getByTitle('Start tracking');
    expect(trackingBtn).toBeInTheDocument();
    expect(trackingBtn).toHaveTextContent('Start');
  });

  test('renders tracking button with stop label when tracking', () => {
    vi.spyOn(useAppStoreModule, 'useAppStore').mockImplementation(() => ({
      ...mockStore,
      isTracking: true,
    }));

    render(<Widget />);

    const trackingBtn = screen.getByTitle('Stop tracking');
    expect(trackingBtn).toHaveTextContent('Stop');
  });

  test('shows idle state when no app is tracked', () => {
    render(<Widget />);

    expect(screen.getByText('Idle')).toBeInTheDocument();
    expect(screen.getByText('None')).toBeInTheDocument();
  });

  test('shows current app name when tracking', () => {
    vi.spyOn(useAppStoreModule, 'useAppStore').mockImplementation(() => ({
      ...mockStore,
      currentApp: 'Visual Studio Code',
    }));

    render(<Widget />);

    expect(screen.getByText('Visual Studio Code')).toBeInTheDocument();
  });

  test('toggle tracking button calls toggleTracking', () => {
    render(<Widget />);

    const trackingBtn = screen.getByTitle('Start tracking');
    fireEvent.click(trackingBtn);

    expect(mockStore.toggleTracking).toHaveBeenCalledTimes(1);
  });

  test('expand button calls setExpanded', () => {
    render(<Widget />);

    const expandBtn = screen.getByTitle('Collapse');
    expect(expandBtn).toHaveTextContent('▼');
    fireEvent.click(expandBtn);

    expect(mockStore.setExpanded).toHaveBeenCalledWith(false);
  });

  test('renders restore icons for collapse, quit, and clear controls', () => {
    render(<Widget />);

    expect(screen.getByTitle('Collapse')).toHaveTextContent('▼');
    expect(screen.getByTitle('Quit app')).toHaveTextContent('✖');
    expect(screen.getByTitle('Clear all data')).toHaveTextContent('🗑');
  });

  test('shows no sessions message when sessions list is empty', () => {
    render(<Widget />);

    expect(screen.getByText('No sessions yet')).toBeInTheDocument();
  });

  test('displays sessions when available', () => {
    const sessions = [
      {
        id: 1,
        app_name: 'Chrome',
        window_title: 'Google',
        start_time: 0,
        end_time: 120,
        duration_seconds: 120,
      },
    ];

    vi.spyOn(useAppStoreModule, 'useAppStore').mockImplementation(() => ({
      ...mockStore,
      sessions,
    }));

    render(<Widget />);

    expect(screen.getByText('Chrome')).toBeInTheDocument();
    expect(screen.getByText('2m 0s')).toBeInTheDocument();
  });

  test('shows app usage section when stats available', () => {
    const stats = {
      total_tracked_seconds: 3600,
      most_used_app: 'Chrome',
      usage_by_app: [
        { app_name: 'Chrome', total_seconds: 2000, percentage: 55.5 },
        { app_name: 'VSCode', total_seconds: 1000, percentage: 27.7 },
      ],
      sessions_count: 5,
    };

    vi.spyOn(useAppStoreModule, 'useAppStore').mockImplementation(() => ({
      ...mockStore,
      stats,
    }));

    render(<Widget />);

    expect(screen.getByText('App Usage')).toBeInTheDocument();
    expect(screen.getByText('33m 20s')).toBeInTheDocument();
  });

  test('displays most used app', () => {
    const stats = {
      total_tracked_seconds: 3600,
      most_used_app: 'Slack',
      usage_by_app: [],
      sessions_count: 0,
    };

    vi.spyOn(useAppStoreModule, 'useAppStore').mockImplementation(() => ({
      ...mockStore,
      stats,
    }));

    render(<Widget />);

    expect(screen.getByText('Slack')).toBeInTheDocument();
  });

  test('shows no data message when no stats exist', () => {
    render(<Widget />);

    expect(screen.getByText('No data yet')).toBeInTheDocument();
  });

  test('shows visible error message when store has an error', () => {
    vi.spyOn(useAppStoreModule, 'useAppStore').mockImplementation(() => ({
      ...mockStore,
      lastError: 'Failed to refresh stats.',
    }));

    render(<Widget />);

    expect(screen.getByRole('alert')).toHaveTextContent('Failed to refresh stats.');
  });
});
