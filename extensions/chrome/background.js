let ws = null;
let backoffMs = 1000;
let eventCounter = 0;
const popupTimers = new Map();
const popupWindows = new Map();
const blockedTabs = new Map();

export function connect() {
  ws = new WebSocket('ws://127.0.0.1:7734');
  ws.onopen = () => {
    backoffMs = 1000;
    sendCurrentActiveTab();
  };
  ws.onmessage = (event) => {
    handleMessage(event.data);
  };
  ws.onclose = () => {
    setTimeout(connect, backoffMs);
    backoffMs = Math.min(backoffMs * 2, 30000);
  };
}

export function sendTabEvent(tab) {
  if (!tab || !tab.id || !tab.url || !ws || ws.readyState !== WebSocket.OPEN) return;
  const payload = {
    type: 'tab_event',
    client_event_id: `chrome-${Date.now()}-${eventCounter++}`,
    source: 'chrome',
    tab_id: tab.id,
    tab_title: tab.title || '',
    tab_url: tab.url,
    timestamp: Math.floor(Date.now() / 1000),
  };
  ws.send(JSON.stringify(payload));
}

export function handleMessage(raw) {
  let decision;
  try {
    decision = JSON.parse(raw);
  } catch {
    return;
  }
  if (decision.type !== 'block_decision' || typeof decision.tab_id !== 'number') return;
  if (decision.blocked) {
    startBlocking(decision.tab_id, decision);
  } else {
    stopBlocking(decision.tab_id);
  }
}

export function startBlocking(tabId, decision) {
  blockedTabs.set(tabId, decision);
  injectOverlay(tabId, decision);
  openDeterrents(tabId, decision);
  clearInterval(popupTimers.get(tabId));
  const interval = Math.max(1000, decision.popup_interval_ms || 5000);
  popupTimers.set(tabId, setInterval(() => openDeterrents(tabId, decision), interval));
}

export function stopBlocking(tabId) {
  blockedTabs.delete(tabId);
  clearInterval(popupTimers.get(tabId));
  popupTimers.delete(tabId);
  removeOverlay(tabId);
  const ids = popupWindows.get(tabId) || new Set();
  ids.forEach((windowId) => chrome.windows.remove(windowId).catch(() => {}));
  popupWindows.delete(tabId);
}

function injectOverlay(tabId, decision) {
  chrome.scripting
    .executeScript({
      target: { tabId },
      files: ['overlay.js'],
    })
    .then(() => {
      chrome.tabs.sendMessage(tabId, {
        type: 'accountability_block',
        category_name: decision.category_name,
        used_seconds: decision.used_seconds,
        limit_seconds: decision.limit_seconds,
      }).catch(() => {});
    })
    .catch(() => {});
}

function removeOverlay(tabId) {
  chrome.tabs.sendMessage(tabId, { type: 'accountability_unblock' }).catch(() => {});
}

function openDeterrents(tabId, decision) {
  chrome.tabs.get(tabId).then((tab) => {
    if (!tab.active || !blockedTabs.has(tabId)) return;
    const ids = popupWindows.get(tabId) || new Set();
    const missing = Math.max(0, 3 - ids.size);
    for (let index = 0; index < missing; index += 1) {
      const page = Math.random() > 0.5 ? 'fake-ad.html' : 'rickroll.html';
      const url = chrome.runtime.getURL(`deterrents/${page}?tabId=${tabId}&category=${encodeURIComponent(decision.category_name || '')}`);
      chrome.windows.create({
        url,
        type: 'popup',
        focused: true,
        width: 420,
        height: 300,
        left: 80 + ids.size * 36,
        top: 80 + ids.size * 36,
      }).then((created) => {
        if (created?.id && blockedTabs.has(tabId)) {
          const latest = popupWindows.get(tabId) || new Set();
          latest.add(created.id);
          popupWindows.set(tabId, latest);
        }
      }).catch(() => {});
    }
    popupWindows.set(tabId, ids);
  }).catch(() => stopBlocking(tabId));
}

function sendCurrentActiveTab() {
  chrome.tabs.query({ active: true, lastFocusedWindow: true }).then((tabs) => {
    if (tabs[0]) sendTabEvent(tabs[0]);
  }).catch(() => {});
}

function handleActivated({ tabId }) {
  Array.from(blockedTabs.keys())
    .filter((blockedTabId) => blockedTabId !== tabId)
    .forEach(stopBlocking);
  chrome.tabs.get(tabId).then(sendTabEvent).catch(() => {});
}

function handleUpdated(_tabId, changeInfo, tab) {
  if (changeInfo.status === 'complete' && tab.active) {
    sendTabEvent(tab);
  }
}

function handleRemoved(tabId) {
  stopBlocking(tabId);
}

function handleWindowRemoved(windowId) {
  popupWindows.forEach((ids, tabId) => {
    ids.delete(windowId);
    if (ids.size === 0) {
      popupWindows.set(tabId, ids);
    }
  });
}

export function registerListeners() {
  chrome.tabs.onActivated.addListener(handleActivated);
  chrome.tabs.onUpdated.addListener(handleUpdated);
  chrome.tabs.onRemoved.addListener(handleRemoved);
  chrome.windows.onRemoved.addListener(handleWindowRemoved);
}

if (globalThis.chrome?.tabs && globalThis.WebSocket) {
  registerListeners();
  connect();
}
