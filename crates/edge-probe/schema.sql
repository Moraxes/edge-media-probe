PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS probe_alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ssrc INTEGER NOT NULL,
    fraction_lost REAL NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
