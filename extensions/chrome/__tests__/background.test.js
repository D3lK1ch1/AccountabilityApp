import { beforeEach, describe, expect, test, vi } from 'vitest';

let listeners;
let windowsCreated;
let wsInstances;

class MockWebSocket {
  static OPEN = 1;

  constructor(url) {
    this.url = url;
    this.readyState = MockWebSocket.OPEN;
    this.sent = [];
    wsInstances.push(this);
  }

  send(message) {
    this.sent.push(message);
  }
}

async function loadModule() {
  vi.resetModules();
  return import('../background.js');
}

beforeEach(() => {
  vi.useFakeTimers();
  listeners = {};
  windowsCreated = [];
  wsInstances = [];
  globalThis.WebSocket = MockWebSocket;
  globalThis.chrome = {
    runtime: {
      getURL: vi.fn((path) => `chrome-extension://id/${path}`),
    },
    tabs: {
      onActivated: { addListener: vi.fn((fn) => { listeners.activated = fn; }) },
      onUpdated: { addListener: vi.fn((fn) => { listeners.updated = fn; }) },
      onRemoved: { addListener: vi.fn((fn) => { listeners.removed = fn; }) },
      get: vi.fn((tabId) => Promise.resolve({ id: tabId, active: true, url: 'https://x.com', title: 'X' })),
      query: vi.fn(() => Promise.resolve([{ id: 7, active: true, url: 'https://x.com', title: 'X' }])),
      sendMessage: vi.fn(() => Promise.resolve()),
    },
    windows: {
      onRemoved: { addListener: vi.fn((fn) => { listeners.windowRemoved = fn; }) },
      create: vi.fn((data) => {
        const created = { id: 100 + windowsCreated.length };
        windowsCreated.push(data);
        return Promise.resolve(created);
      }),
      remove: vi.fn(() => Promise.resolve()),
    },
    scripting: {
      executeScript: vi.fn(() => Promise.resolve()),
    },
  };
});

describe('chrome background', () => {
  test('connects and sends current tab event', async () => {
    const mod = await loadModule();
    mod.connect();
    wsInstances.at(-1).onopen();
    await Promise.resolve();

    const payload = JSON.parse(wsInstances.at(-1).sent[0]);

    expect(payload.source).toBe('chrome');
    expect(payload.type).toBe('tab_event');
    expect(payload.tab_id).toBe(7);
  });

  test('does not send when websocket is not open', async () => {
    const mod = await loadModule();
    mod.connect();
    wsInstances.at(-1).readyState = 0;
    mod.sendTabEvent({ id: 1, url: 'https://instagram.com', title: 'Instagram' });

    expect(wsInstances.at(-1).sent).toEqual([]);
  });

  test('starts blocking from block decision', async () => {
    const mod = await loadModule();
    mod.handleMessage(JSON.stringify({
      type: 'block_decision',
      tab_id: 2,
      blocked: true,
      category_name: 'Social Media',
      used_seconds: 3600,
      limit_seconds: 3600,
      popup_interval_ms: 5000,
    }));
    await Promise.resolve();
    await Promise.resolve();

    expect(chrome.scripting.executeScript).toHaveBeenCalledWith({
      target: { tabId: 2 },
      files: ['overlay.js'],
    });
    expect(chrome.windows.create).toHaveBeenCalledTimes(3);
  });

  test('stops blocking when decision is not blocked', async () => {
    const mod = await loadModule();
    mod.startBlocking(3, {
      category_name: 'Social Media',
      used_seconds: 3600,
      limit_seconds: 3600,
      popup_interval_ms: 5000,
    });
    await Promise.resolve();
    await Promise.resolve();

    mod.handleMessage(JSON.stringify({ type: 'block_decision', tab_id: 3, blocked: false }));

    expect(chrome.tabs.sendMessage).toHaveBeenCalledWith(3, { type: 'accountability_unblock' });
  });

  test('registers tab and window listeners', async () => {
    await loadModule();

    expect(chrome.tabs.onActivated.addListener).toHaveBeenCalled();
    expect(chrome.tabs.onUpdated.addListener).toHaveBeenCalled();
    expect(chrome.tabs.onRemoved.addListener).toHaveBeenCalled();
    expect(chrome.windows.onRemoved.addListener).toHaveBeenCalled();
  });

  test('activation sends tab event', async () => {
    await loadModule();
    wsInstances.at(-1).onopen();
    await Promise.resolve();
    listeners.activated({ tabId: 9 });
    await Promise.resolve();

    const payload = JSON.parse(wsInstances.at(-1).sent.at(-1));

    expect(payload.tab_id).toBe(9);
  });

  test('caps popup windows at 3 across repeated deterrent ticks', async () => {
    const mod = await loadModule();
    mod.startBlocking(4, {
      category_name: 'Games',
      used_seconds: 1800,
      limit_seconds: 1800,
      popup_interval_ms: 5000,
    });
    await Promise.resolve();
    await Promise.resolve();
    expect(chrome.windows.create).toHaveBeenCalledTimes(3);

    await vi.advanceTimersByTimeAsync(5000);

    expect(chrome.windows.create).toHaveBeenCalledTimes(3);
  });

  test('only opens fake-ad or rickroll deterrent pages', async () => {
    const mod = await loadModule();
    mod.startBlocking(5, {
      category_name: 'Social Media',
      used_seconds: 60,
      limit_seconds: 60,
      popup_interval_ms: 5000,
    });
    await Promise.resolve();
    await Promise.resolve();

    expect(windowsCreated).toHaveLength(3);
    windowsCreated.forEach((data) => {
      expect(data.url).toMatch(/deterrents\/(fake-ad|rickroll)\.html\?/);
    });
  });

  test('does not stop blocking when activating a tab inside a deterrent popup window', async () => {
    const mod = await loadModule();
    wsInstances.at(-1).onopen();
    await Promise.resolve();

    mod.startBlocking(5, {
      category_name: 'Social Media',
      used_seconds: 3600,
      limit_seconds: 3600,
      popup_interval_ms: 5000,
    });
    // let openDeterrents resolve: tabs.get → windows.create × 3 → .then callbacks
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();

    // deterrent windows are now 100, 101, 102 — simulate activating a tab inside window 100
    chrome.tabs.get.mockResolvedValueOnce({
      id: 99, windowId: 100, active: true,
      url: 'chrome-extension://id/deterrents/fake-ad.html', title: 'Ad',
    });

    listeners.activated({ tabId: 99 });
    await Promise.resolve();

    // stopBlocking(5) must NOT have fired — overlay unblock message should not have been sent
    expect(chrome.tabs.sendMessage).not.toHaveBeenCalledWith(5, { type: 'accountability_unblock' });
  });

  test('sends a heartbeat tab event every 15 seconds while connected', async () => {
    const mod = await loadModule();
    mod.connect();
    const socket = wsInstances.at(-1);
    socket.onopen();
    await Promise.resolve();

    const sentAfterOpen = socket.sent.length; // 1 event from sendCurrentActiveTab on open

    await vi.advanceTimersByTimeAsync(15000);
    await Promise.resolve();

    expect(socket.sent.length).toBeGreaterThan(sentAfterOpen);
    const heartbeatPayload = JSON.parse(socket.sent.at(-1));
    expect(heartbeatPayload.source).toBe('chrome');
    expect(heartbeatPayload.type).toBe('tab_event');
  });

  test('clears heartbeat timer when websocket closes', async () => {
    const mod = await loadModule();
    mod.connect();
    const socket = wsInstances.at(-1);
    socket.onopen();
    await Promise.resolve();

    const sentAfterOpen = socket.sent.length;

    socket.onclose();

    // advance past one full heartbeat interval — should not fire
    await vi.advanceTimersByTimeAsync(15000);
    await Promise.resolve();

    expect(socket.sent.length).toBe(sentAfterOpen);
  });

  test('opens a replacement popup after one is closed externally', async () => {
    const mod = await loadModule();
    mod.startBlocking(6, {
      category_name: 'Games',
      used_seconds: 1800,
      limit_seconds: 1800,
      popup_interval_ms: 5000,
    });
    await Promise.resolve();
    await Promise.resolve();
    expect(chrome.windows.create).toHaveBeenCalledTimes(3);

    listeners.windowRemoved(100);
    await vi.advanceTimersByTimeAsync(5000);

    expect(chrome.windows.create).toHaveBeenCalledTimes(4);
  });
});
