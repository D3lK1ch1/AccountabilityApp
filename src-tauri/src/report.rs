use crate::database::{AppSession, TabSession};
use chrono::{Local, TimeZone};

struct ReportEntry {
    start_time: i64,
    end_time: Option<i64>,
    duration_seconds: i64,
    source: String,
    detail: String,
}

fn fmt_time(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "?".to_string())
}

fn fmt_date(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn fmt_duration(secs: i64) -> String {
    let secs = secs.max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

pub fn format_report(
    window_start: i64,
    window_end: i64,
    app_sessions: &[AppSession],
    tab_sessions: &[TabSession],
) -> String {
    let mut entries: Vec<ReportEntry> = Vec::new();

    for s in app_sessions {
        entries.push(ReportEntry {
            start_time: s.start_time,
            end_time: s.end_time,
            duration_seconds: s.duration_seconds,
            source: format!("app: {}", s.app_name),
            detail: s.window_title.clone().unwrap_or_default(),
        });
    }

    for s in tab_sessions {
        entries.push(ReportEntry {
            start_time: s.start_time,
            end_time: s.end_time,
            duration_seconds: s.duration_seconds,
            source: s.source.clone(),
            detail: format!("{} — {}", s.tab_title, s.tab_url),
        });
    }

    entries.sort_by_key(|e| e.start_time);

    let elapsed = (window_end - window_start).max(0);
    let tracked_total: i64 = entries.iter().map(|e| e.duration_seconds.max(0)).sum();

    let mut out = String::new();
    out.push_str(&format!("# Session Report — {}\n\n", fmt_date(window_start)));
    out.push_str(&format!(
        "**Window:** {} – {} ({} elapsed)\n\n",
        fmt_time(window_start),
        fmt_time(window_end),
        fmt_duration(elapsed)
    ));

    if entries.is_empty() {
        out.push_str("_No tracked activity in this window._\n");
    } else {
        out.push_str("| Time | Duration | Source | Detail |\n");
        out.push_str("|------|----------|--------|--------|\n");
        for e in &entries {
            let end_str = match e.end_time {
                Some(t) => fmt_time(t),
                None => "open".to_string(),
            };
            out.push_str(&format!(
                "| {}\u{2013}{} | {} | {} | {} |\n",
                fmt_time(e.start_time),
                end_str,
                fmt_duration(e.duration_seconds),
                e.source,
                e.detail.replace('|', "\\|"),
            ));
        }
    }

    out.push_str(&format!(
        "\n---\n{} entries. Tracked switch-time: {} vs {} elapsed.\n",
        entries.len(),
        fmt_duration(tracked_total),
        fmt_duration(elapsed)
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_session(start: i64, end: i64, name: &str) -> AppSession {
        AppSession {
            id: Some(1),
            app_name: name.to_string(),
            window_title: Some("Window".to_string()),
            start_time: start,
            end_time: Some(end),
            duration_seconds: end - start,
        }
    }

    fn tab_session(start: i64, end: i64, source: &str) -> TabSession {
        TabSession {
            id: Some(1),
            source: source.to_string(),
            tab_url: "https://example.com".to_string(),
            tab_title: "Example".to_string(),
            start_time: start,
            end_time: Some(end),
            duration_seconds: end - start,
        }
    }

    #[test]
    fn merges_and_sorts_by_start_time() {
        let apps = vec![app_session(200, 300, "Terminal")];
        let tabs = vec![tab_session(100, 200, "chrome")];

        let report = format_report(100, 300, &apps, &tabs);

        let chrome_pos = report.find("chrome").unwrap();
        let terminal_pos = report.find("Terminal").unwrap();
        assert!(chrome_pos < terminal_pos, "chrome entry should appear before terminal entry");
    }

    #[test]
    fn empty_window_says_so() {
        let report = format_report(100, 200, &[], &[]);
        assert!(report.contains("No tracked activity"));
    }

    #[test]
    fn footer_reports_entry_count() {
        let apps = vec![app_session(100, 150, "A"), app_session(150, 200, "B")];
        let report = format_report(100, 200, &apps, &[]);
        assert!(report.contains("2 entries"));
    }
}
