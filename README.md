# Accountability App -

**Type:** Cross-platform Desktop/Mobile Application  
**Core Feature:** Tracks user app/browser usage, sleep patterns monitors, and provides an AI therapist chatbot to help break bad habits

## Tech Stack Recommendations

| Component | Recommended Options |
|-----------|---------------------|
| **Framework** | Tauri + React |
| **State Management** | Zustand |
| **Backend** | Rust |
| **Database** | PostgreSQL |
| **AI Integration** |local LLM (Ollama) |
| **Activity Tracking** | See Platform-Specific APIs below |

---

## Platform-Specific APIs for Activity Tracking

### Windows
- `Windows.ActivityHistory` API (Windows 10+)
- `PyWin32` / `psutil` for process monitoring
- [Windows Docs: Activity History](https://learn.microsoft.com/en-us/windows/apps/activity-activity-history)


### iOS
- Screen Time API (requires special entitlements)
- [Apple Developer: Screen Time](https://developer.apple.com/documentation/screentime)

### Browser Extensions (for browser tab tracking)
- Chrome: [Extensions API - tabs](https://developer.chrome.com/docs/extensions/reference/tabs)
- Firefox: [WebExtensions API](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions)

---

## Core Features
- Behind-the-scenes app that tracks user app/browser usage and provides an AI therapist chatbot to help break bad habits
- Gives an option to do ad pop ups on apps and social media sites within a timeframe of user's choosing, essentially blocking user from using those apps and closing it, returning to productivity
- AI therapist chatbot provides personalized advice based on user's usage patterns and habits, to figure out what can be done to break bad habits and improve productivity

---


## Useful Libraries

| Purpose | Library |
|---------|---------|
| Window tracking | `active-win` |
| Process monitoring | `psutil`, `node-ps99` |
| Database | `better-sqlite3`, `prisma`, `drizzle-orm` |
| Charts | `recharts`, `chart.js`, `nivo` |
| AI Chat UI | `react-markdown`, `chat-ui` |
| Date handling | `date-fns`, `dayjs` |
| Notifications | `node-notifier`, `electron-notification` |

---

## Security Considerations

1. **Local Data Storage:** Encrypt database with user-derived key
2. **Network:** Use HTTPS for all API calls
3. **API Keys:** Never expose in frontend code; use backend proxy
4. **Permissions:** Request only necessary permissions
5. **Data Minimization:** Don't collect more than needed

---

## Current Progress
A widget showing app usage, such as Google, File Explorer, VSCode and any other app. Not able to show the tabs from Chrome used. Acts as a time tracker, but unsure how to show heavy use of data. Haven't stored into database for memories. When using it for a while, suddenly disappeared without warning. To work on.


Feedback is always welcomed, as I go through this ReadME and figure out what works and doesn't work.