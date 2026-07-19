use std::collections::HashMap;
use std::time::Instant;

const CACHE_MAX_SIZE: usize = 100;
const CACHE_TTL_SECS: u64 = 300;

#[derive(Debug, Clone)]
pub struct FtsSearchResult {
    pub book_id: i64,
    pub title: String,
    pub author: String,
    pub category: String,
    pub word_count: i64,
    pub created_at: i64,
    pub relevance_score: f64,
    pub title_matches: i64,
    pub author_matches: i64,
    pub content_matches: i64,
    pub snippet: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchField {
    All,
    Title,
    Author,
    Content,
}

impl SearchField {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "all" => Some(SearchField::All),
            "title" => Some(SearchField::Title),
            "author" => Some(SearchField::Author),
            "content" => Some(SearchField::Content),
            _ => None,
        }
    }
}

pub struct SearchCache {
    cache: HashMap<String, (Vec<FtsSearchResult>, Instant)>,
    max_size: usize,
    ttl_secs: u64,
}

impl SearchCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_size: CACHE_MAX_SIZE,
            ttl_secs: CACHE_TTL_SECS,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<Vec<FtsSearchResult>> {
        if let Some((results, timestamp)) = self.cache.get(key) {
            if timestamp.elapsed().as_secs() < self.ttl_secs {
                return Some(results.clone());
            }
        }
        None
    }

    pub fn put(&mut self, key: String, results: Vec<FtsSearchResult>) {
        if self.cache.len() >= self.max_size {
            if let Some(oldest_key) = self
                .cache
                .iter()
                .min_by_key(|(_, (_, ts))| ts.elapsed())
                .map(|(k, _)| k.clone())
            {
                self.cache.remove(&oldest_key);
            }
        }
        self.cache.insert(key, (results, Instant::now()));
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }
}

pub fn build_fts_match_expr(keyword: &str, _exact: bool, search_field: SearchField) -> String {
    let match_expr = format!("simple_query('{}')", escape_single_quote(keyword));

    match search_field {
        SearchField::All => format!("books_fts MATCH {}", match_expr),
        SearchField::Title => {
            format!("books_fts MATCH ('title : ' || {})", match_expr)
        }
        SearchField::Author => {
            format!("books_fts MATCH ('author : ' || {})", match_expr)
        }
        SearchField::Content => {
            format!("books_fts MATCH ('content : ' || {})", match_expr)
        }
    }
}

fn build_or_expr(tokens: &[String]) -> String {
    if tokens.is_empty() {
        return "simple_query('')".to_string();
    }
    let parts: Vec<String> = tokens
        .iter()
        .map(|t| format!("simple_query('{}')", escape_single_quote(t)))
        .collect();
    format!("({})", parts.join(" OR "))
}

fn build_phrase_expr(keyword: &str) -> String {
    format!("simple_query('{}')", escape_single_quote(&keyword))
}

fn build_prefix_expr(keyword: &str) -> String {
    return format!("simple_query('{}')", escape_single_quote(keyword));
}

pub fn tokenize_chinese(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();

    for ch in text.chars() {
        if ch.is_whitespace() {
            if !current_token.is_empty() && current_token.len() >= 2 {
                tokens.push(current_token.clone());
                current_token.clear();
            }
        } else if (ch as u32) > 0x4E00 && (ch as u32) < 0x9FFF {
            if !current_token.is_empty() && !is_chinese(current_token.chars().last().unwrap()) {
                if current_token.len() >= 2 {
                    tokens.push(current_token.clone());
                }
                current_token.clear();
            }
            current_token.push(ch);
        } else if ch.is_alphanumeric() {
            if !current_token.is_empty() && is_chinese(current_token.chars().last().unwrap()) {
                if current_token.len() >= 2 {
                    tokens.push(current_token.clone());
                }
                current_token.clear();
            }
            current_token.push(ch.to_lowercase().next().unwrap());
        } else {
            if !current_token.is_empty() && current_token.len() >= 2 {
                tokens.push(current_token.clone());
                current_token.clear();
            }
        }
    }

    if !current_token.is_empty() && current_token.len() >= 2 {
        tokens.push(current_token);
    }

    tokens
}

fn is_chinese(ch: char) -> bool {
    let cp = ch as u32;
    (cp >= 0x4E00 && cp <= 0x9FFF)
        || (cp >= 0x3400 && cp <= 0x4DBF)
        || (cp >= 0x20000 && cp <= 0x2A6DF)
}

fn escape_single_quote(s: &str) -> String {
    s.replace('\'', "''")
}

pub fn normalize_bm25_score(raw_score: f64, min_score: f64, max_score: f64) -> f64 {
    if (max_score - min_score).abs() < f64::EPSILON {
        if raw_score < 0.0 {
            100.0
        } else {
            50.0
        }
    } else {
        let normalized = (raw_score - min_score) / (max_score - min_score);
        (normalized * 100.0).clamp(0.0, 100.0)
    }
}

pub fn highlight_snippet(snippet: &str, start_marker: &str, end_marker: &str) -> String {
    snippet
        .replace(start_marker, "\x1b[33m")
        .replace(end_marker, "\x1b[0m")
}

pub fn strip_highlight_markers(snippet: &str, start_marker: &str, end_marker: &str) -> String {
    snippet.replace(start_marker, "").replace(end_marker, "")
}

pub fn count_matches(text: &str, start_marker: &str, end_marker: &str) -> i64 {
    let starts = text.matches(start_marker).count() as i64;
    let ends = text.matches(end_marker).count() as i64;
    starts.min(ends)
}

pub fn format_search_results(
    results: &[FtsSearchResult],
    keyword: &str,
    search_field: SearchField,
) {
    if results.is_empty() {
        println!("未找到与 \"{}\" 相关的内容", keyword);
        return;
    }

    println!(
        "搜索结果（共 {} 条，搜索范围: {}）：\n",
        results.len(),
        field_display_name(search_field)
    );

    for (idx, result) in results.iter().enumerate() {
        let score_bar = build_score_bar(result.relevance_score);
        let total_matches = result.title_matches + result.author_matches + result.content_matches;

        let field_dist = build_field_distribution(
            result.title_matches,
            result.author_matches,
            result.content_matches,
        );

        println!("┌─────────────────────────────────────────────────────────");
        println!("│ \x1b[1m{}. {}\x1b[0m", idx + 1, result.title);
        println!(
            "│ 作者: {} | 分类: {} | 字数: {}",
            result.author, result.category, result.word_count
        );
        println!("│ 相关度: {} {:.0}/100", score_bar, result.relevance_score);
        println!("│ 匹配: 共{}处 {}", total_matches, field_dist);
        println!("│ 更新: {}", result.created_at);
        println!("│ ─────────────────────────────────────────────────────");

        if !result.snippet.is_empty() {
            let lines: Vec<&str> = result.snippet.lines().collect();
            for line in lines.iter().take(3) {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    println!("│   {}", trimmed);
                }
            }
        }

        println!("└─────────────────────────────────────────────────────────\n");
    }
}

fn build_score_bar(score: f64) -> String {
    let filled = (score / 10.0).round() as usize;
    let empty = 10 - filled.min(10);
    let bar: String = "█".repeat(filled.min(10)) + &"░".repeat(empty);
    bar
}

fn build_field_distribution(title_m: i64, author_m: i64, content_m: i64) -> String {
    let mut parts = Vec::new();
    if title_m > 0 {
        parts.push(format!("标题:{}", title_m));
    }
    if author_m > 0 {
        parts.push(format!("作者:{}", author_m));
    }
    if content_m > 0 {
        parts.push(format!("内容:{}", content_m));
    }
    if parts.is_empty() {
        "无详细分布".to_string()
    } else {
        format!("({})", parts.join(" "))
    }
}

fn field_display_name(field: SearchField) -> String {
    match field {
        SearchField::All => "全部字段".to_string(),
        SearchField::Title => "标题".to_string(),
        SearchField::Author => "作者".to_string(),
        SearchField::Content => "内容".to_string(),
    }
}

pub struct IndexUpdateQueue {
    queue: Vec<i64>,
    batch_size: usize,
}

impl IndexUpdateQueue {
    pub fn new(batch_size: usize) -> Self {
        Self {
            queue: Vec::new(),
            batch_size,
        }
    }

    pub fn enqueue(&mut self, book_id: i64) {
        if !self.queue.contains(&book_id) {
            self.queue.push(book_id);
        }
    }

    pub fn is_ready(&self) -> bool {
        self.queue.len() >= self.batch_size
    }

    pub fn drain(&mut self) -> Vec<i64> {
        let items = self.queue.clone();
        self.queue.clear();
        items
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_chinese() {
        let tokens = tokenize_chinese("斗破苍穹");
        assert!(!tokens.is_empty());

        let tokens = tokenize_chinese("hello world");
        assert_eq!(tokens, vec!["hello", "world"]);

        let tokens = tokenize_chinese("斗破苍穹 hello");
        assert!(tokens.len() >= 2);
    }

    #[test]
    fn test_tokenize_mixed() {
        let tokens = tokenize_chinese("天蚕土豆");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_normalize_bm25_score() {
        let score = normalize_bm25_score(-5.0, -10.0, -1.0);
        assert!(score > 0.0 && score <= 100.0);

        let score_same = normalize_bm25_score(-5.0, -5.0, -5.0);
        assert_eq!(score_same, 100.0);
    }

    #[test]
    fn test_count_matches() {
        let text = ">>>hello<<< world >>>test<<<";
        assert_eq!(count_matches(text, ">>>", "<<<"), 2);

        let text_no_match = "hello world";
        assert_eq!(count_matches(text_no_match, ">>>", "<<<"), 0);
    }

    #[test]
    fn test_highlight_snippet() {
        let snippet = ">>>斗破<<<苍穹";
        let result = highlight_snippet(snippet, ">>>", "<<<");
        assert!(result.contains("\x1b[33m"));
        assert!(result.contains("\x1b[0m"));
    }

    #[test]
    fn test_strip_highlight_markers() {
        let snippet = ">>>斗破<<<苍穹";
        let result = strip_highlight_markers(snippet, ">>>", "<<<");
        assert_eq!(result, "斗破苍穹");
    }

    #[test]
    fn test_build_fts_match_expr_simple() {
        let expr = build_fts_match_expr("斗破", false, SearchField::All);
        assert!(expr.contains("simple_query"));
        assert!(expr.contains("books_fts MATCH"));
    }

    #[test]
    fn test_build_fts_match_expr_column_filter() {
        let expr = build_fts_match_expr("斗破", false, SearchField::Title);
        assert!(expr.contains("title :"));
    }

    #[test]
    fn test_search_field_from_str() {
        assert_eq!(SearchField::from_str("all"), Some(SearchField::All));
        assert_eq!(SearchField::from_str("title"), Some(SearchField::Title));
        assert_eq!(SearchField::from_str("author"), Some(SearchField::Author));
        assert_eq!(SearchField::from_str("content"), Some(SearchField::Content));
        assert_eq!(SearchField::from_str("invalid"), None);
    }

    #[test]
    fn test_index_update_queue() {
        let mut queue = IndexUpdateQueue::new(3);
        assert!(!queue.is_ready());

        queue.enqueue(1);
        queue.enqueue(2);
        assert!(!queue.is_ready());

        queue.enqueue(3);
        assert!(queue.is_ready());

        let items = queue.drain();
        assert_eq!(items.len(), 3);
        assert!(queue.queue.is_empty());
    }

    #[test]
    fn test_index_update_queue_no_duplicates() {
        let mut queue = IndexUpdateQueue::new(3);
        queue.enqueue(1);
        queue.enqueue(1);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_search_cache() {
        let mut cache = SearchCache::new();
        assert!(cache.get("test").is_none());

        let results = vec![FtsSearchResult {
            book_id: 1,
            title: "test".to_string(),
            author: "author".to_string(),
            category: "cat".to_string(),
            word_count: 100,
            created_at: 0,
            relevance_score: 95.0,
            title_matches: 1,
            author_matches: 0,
            content_matches: 5,
            snippet: "test snippet".to_string(),
        }];

        cache.put("test".to_string(), results.clone());
        let cached = cache.get("test");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[test]
    fn test_escape_single_quote() {
        assert_eq!(escape_single_quote("it's"), "it''s");
        assert_eq!(escape_single_quote("normal"), "normal");
    }

    #[test]
    fn test_build_score_bar() {
        let bar = build_score_bar(80.0);
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));

        let bar_zero = build_score_bar(0.0);
        assert_eq!(bar_zero, "░░░░░░░░░░");
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize_chinese("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_special_chars() {
        let tokens = tokenize_chinese("hello, world! 你好");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_format_search_results_empty() {
        format_search_results(&[], "test", SearchField::All);
    }

    #[test]
    fn test_format_search_results_with_data() {
        let results = vec![FtsSearchResult {
            book_id: 1,
            title: "测试小说".to_string(),
            author: "测试作者".to_string(),
            category: "玄幻".to_string(),
            word_count: 100000,
            created_at: 0,
            relevance_score: 85.0,
            title_matches: 2,
            author_matches: 1,
            content_matches: 10,
            snippet: "这是>>>测试<<<内容".to_string(),
        }];
        format_search_results(&results, "测试", SearchField::All);
    }
}
