//! SQL schema definitions for the banzhu database.

pub(crate) const CREATE_BOOKS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    website_book_id INTEGER UNIQUE,
    path_num INTEGER NOT NULL DEFAULT 0,
    title TEXT NOT NULL,
    filename TEXT NOT NULL,
    author TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    introduce TEXT NOT NULL DEFAULT '',
    likes INTEGER NOT NULL DEFAULT 0 CHECK(likes >= 0),
    word_count INTEGER NOT NULL DEFAULT 0 CHECK(word_count >= 0),
    page_count INTEGER NOT NULL DEFAULT 0 CHECK(page_count >= 0),
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
)"#;

pub(crate) const CREATE_CHAPTERS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS chapters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    url TEXT NOT NULL DEFAULT '',
    chapter_order INTEGER NOT NULL CHECK(chapter_order > 0),
    word_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE,
    UNIQUE(book_id, chapter_order)
)"#;

pub(crate) const CREATE_SECTIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS sections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chapter_id INTEGER NOT NULL,
    book_id INTEGER NOT NULL,
    url TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    section_order INTEGER NOT NULL CHECK(section_order > 0),
    FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE,
    UNIQUE(chapter_id, section_order)
)"#;

pub(crate) const CREATE_BOOKSHELF_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS bookshelf (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL UNIQUE,
    group_name TEXT NOT NULL DEFAULT 'reading'
        CHECK(group_name IN ('reading','want','finished')),
    added_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
)"#;

pub(crate) const CREATE_READING_PROGRESS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS reading_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL UNIQUE,
    chapter_order INTEGER NOT NULL DEFAULT 1,
    page_index INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
)"#;

pub(crate) const CREATE_CRAWL_LOGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS crawl_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    level TEXT NOT NULL DEFAULT 'INFO'
        CHECK(level IN ('DEBUG','INFO','WARN','ERROR')),
    message TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
)"#;

/// 每本书的爬取任务状态记录
/// - status: pending(待爬取) / running(爬取中) / success(成功) / failed(失败) / skipped(跳过)
/// - progress: 0-100 百分比
/// - error_message: 失败原因（仅 failed 状态有值）
/// - chapters_total / chapters_done: 章节总数与已完成数（用于精细进度）
pub(crate) const CREATE_CRAWL_TASKS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS crawl_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    website_book_id INTEGER NOT NULL,
    book_id INTEGER,
    title TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','running','success','failed','skipped')),
    progress INTEGER NOT NULL DEFAULT 0 CHECK(progress >= 0 AND progress <= 100),
    chapters_total INTEGER NOT NULL DEFAULT 0,
    chapters_done INTEGER NOT NULL DEFAULT 0,
    error_message TEXT NOT NULL DEFAULT '',
    trigger TEXT NOT NULL DEFAULT 'manual'
        CHECK(trigger IN ('manual','cron','retry')),
    started_at INTEGER,
    finished_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(website_book_id)
)"#;

pub(crate) const CREATE_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_chapters_book_id ON chapters(book_id);
CREATE INDEX IF NOT EXISTS idx_sections_chapter_id ON sections(chapter_id);
CREATE INDEX IF NOT EXISTS idx_sections_book_id ON sections(book_id);
CREATE INDEX IF NOT EXISTS idx_crawl_logs_created ON crawl_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_crawl_tasks_status ON crawl_tasks(status);
CREATE INDEX IF NOT EXISTS idx_crawl_tasks_updated ON crawl_tasks(updated_at DESC);
"#;

pub(crate) const CREATE_FTS_TABLE: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS books_fts USING fts5(
    title,
    author,
    content,
    tokenize='simple disable_pinyin'
);
"#;

pub(crate) const CREATE_READING_SESSIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS reading_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL,
    chapter_order INTEGER NOT NULL,
    duration_sec INTEGER NOT NULL CHECK(duration_sec > 0),
    chapters_read INTEGER NOT NULL DEFAULT 0,
    started_at INTEGER NOT NULL,
    ended_at INTEGER NOT NULL,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_reading_sessions_book ON reading_sessions(book_id);
CREATE INDEX IF NOT EXISTS idx_reading_sessions_started ON reading_sessions(started_at DESC);
"#;

pub(crate) const CREATE_READING_GOALS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS reading_goals (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    daily_minutes INTEGER NOT NULL DEFAULT 30 CHECK(daily_minutes >= 0),
    daily_chapters INTEGER NOT NULL DEFAULT 5 CHECK(daily_chapters >= 0),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
INSERT OR IGNORE INTO reading_goals (id) VALUES (1);
"#;

pub(crate) const ALTER_READING_PROGRESS_LAST_READ: &str =
    "ALTER TABLE reading_progress ADD COLUMN last_read_at INTEGER NOT NULL DEFAULT 0;";
