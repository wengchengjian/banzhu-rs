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

pub(crate) const CREATE_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_chapters_book_id ON chapters(book_id);
CREATE INDEX IF NOT EXISTS idx_sections_chapter_id ON sections(chapter_id);
CREATE INDEX IF NOT EXISTS idx_sections_book_id ON sections(book_id);
CREATE INDEX IF NOT EXISTS idx_crawl_logs_created ON crawl_logs(created_at DESC);
"#;

pub(crate) const CREATE_FTS_TABLE: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS books_fts USING fts5(
    title,
    author,
    content,
    tokenize='simple disable_pinyin'
);
"#;
